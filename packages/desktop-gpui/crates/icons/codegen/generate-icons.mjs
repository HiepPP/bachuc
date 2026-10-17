#!/usr/bin/env node
// Writes one lucide SVG per icon that packages/app/src imports from "lucide-react-native", and
// the Rust name table `src/names.rs`. Run it again when the app imports a new icon.
// Usage: node generate-icons.mjs
// Only reads node_modules and packages/app/src; output is deterministic, so a second run
// changes nothing.

import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const crateDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = join(crateDir, "..", "..", "..", "..");
const appSrc = join(repoRoot, "packages", "app", "src");
const lucideDir = join(repoRoot, "node_modules", "lucide-react-native", "dist", "esm");
const assetsDir = join(crateDir, "assets");
const namesFile = join(crateDir, "src", "names.rs");

// Exports of lucide-react-native that are not icons.
const NON_ICON_NAMES = new Set(["Icon", "LucideIcon", "LucideProps", "createLucideIcon", "icons"]);
// plugins/icons.ts imports the whole package by namespace; plugin surfaces are out of scope.
const SKIPPED_FILES = new Set([join(appSrc, "plugins", "icons.ts")]);

// Icons that the GPUI plugin ports draw by their lucide export name; the Electron app resolves
// them at runtime through the namespace import, so no import in packages/app/src lists them.
const PLUGIN_ICONS = [
  // TASK-187, jev-orchestrator
  "Route",
  // TASK-189 and TASK-193, board
  "Columns3",
  "CornerLeftUp",
  "CornerDownRight",
  "Laptop",
  "PencilOff",
  // TASK-194, watchtower-board
  "Scale",
  "ClipboardCheck",
  "MessageCircleQuestion",
  "Hand",
  "LoaderCircle",
  // TASK-181, TASK-183, TASK-185, and TASK-186
  "Quote",
  "MessagesSquare",
  "Power",
  "Languages",
  "Send",
  "FileImage",
  "FileCode",
  "CloudDownload",
];

function walk(dir, files = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "node_modules") walk(path, files);
    } else if (/\.(ts|tsx)$/.test(entry.name)) {
      files.push(path);
    }
  }
  return files;
}

function collectImportedNames() {
  const names = new Set();
  const importPattern =
    /(?:import|export)\s+(type\s+)?(?:[A-Za-z_$][\w$]*\s*,\s*)?\{([^}]*)\}\s*from\s*["']lucide-react-native["']/g;
  for (const file of walk(appSrc)) {
    if (SKIPPED_FILES.has(file)) continue;
    const source = readFileSync(file, "utf8");
    for (const match of source.matchAll(importPattern)) {
      if (match[1]) continue;
      for (const specifier of match[2].split(",")) {
        const text = specifier.trim();
        if (!text || text.startsWith("type ")) continue;
        names.add(text.split(/\s+as\s+/)[0].trim());
      }
    }
  }
  return names;
}

// `export { default as A, default as B } from './icons/a.js';` gives name -> icon file.
function readNameIndex() {
  const index = new Map();
  const source = readFileSync(join(lucideDir, "lucide-react-native.js"), "utf8");
  for (const match of source.matchAll(/export\s*\{([^}]*)\}\s*from\s*'\.\/icons\/([^']+)\.js'/g)) {
    for (const specifier of match[1].split(",")) {
      const name = specifier.trim().split(/\s+as\s+/)[1];
      if (name) index.set(name.trim(), match[2]);
    }
  }
  return index;
}

// The icon file holds a JS array literal of [tag, attributes]; quoting the keys makes it JSON.
function readNodes(file) {
  const source = readFileSync(join(lucideDir, "icons", `${file}.js`), "utf8");
  const start = source.indexOf("createLucideIcon(");
  const open = source.indexOf("[", source.indexOf(",", start));
  const close = source.lastIndexOf("]);");
  if (start < 0 || open < 0 || close < open) throw new Error(`cannot read nodes of ${file}`);
  const json = source.slice(open, close + 1).replace(/([{,]\s*)([A-Za-z_][\w]*)\s*:/g, '$1"$2":');
  return JSON.parse(json);
}

const kebab = (name) => name.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();

function renderSvg(nodes) {
  const lines = nodes.map(([tag, attributes]) => {
    const rendered = Object.entries(attributes)
      .filter(([key]) => key !== "key")
      .map(([key, value]) => `${kebab(key)}="${value}"`)
      .join(" ");
    return `  <${tag} ${rendered} />`;
  });
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">',
    ...lines,
    "</svg>",
    "",
  ].join("\n");
}

function writeIfChanged(path, content) {
  if (existsSync(path) && readFileSync(path, "utf8") === content) return false;
  writeFileSync(path, content);
  return true;
}

function renderNames(entries) {
  // Rust compares bytes, so sort the full path ("a-b.svg" sorts before "a.svg").
  const files = [...new Set(entries.map((entry) => entry.file))].sort((a, b) =>
    `${a}.svg` < `${b}.svg` ? -1 : 1,
  );
  const variants = entries.map((entry) => `    ${entry.name},`).join("\n");
  const all = entries.map((entry) => `        IconName::${entry.name},`).join("\n");
  const arms = entries
    .map((entry) => `            IconName::${entry.name} => "lucide/${entry.file}.svg",`)
    .join("\n");
  const table = files
    .map((file) => `    ("lucide/${file}.svg", include_bytes!("../assets/${file}.svg")),`)
    .join("\n");
  return `// Generated by codegen/generate-icons.mjs. Do not edit.

/// One variant per lucide icon that the Electron app imports, named as the lucide import.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconName {
${variants}
}

impl IconName {
    pub const ALL: &'static [IconName] = &[
${all}
    ];

    /// The asset path that \`Assets\` serves.
    pub fn path(self) -> &'static str {
        match self {
${arms}
        }
    }
}

/// Sorted by path, so \`Assets::load\` can binary search.
pub(crate) static SVGS: &[(&str, &[u8])] = &[
${table}
];
`;
}

const index = readNameIndex();
const imported = [...new Set([...collectImportedNames(), ...PLUGIN_ICONS])].sort();
const skipped = imported.filter((name) => NON_ICON_NAMES.has(name));
const missing = imported.filter((name) => !NON_ICON_NAMES.has(name) && !index.has(name));
if (missing.length > 0) {
  console.error(`missing in lucide-react-native: ${missing.join(", ")}`);
  process.exit(1);
}

const entries = imported
  .filter((name) => !NON_ICON_NAMES.has(name))
  .map((name) => ({ name, file: index.get(name) }));
const files = [...new Set(entries.map((entry) => entry.file))].sort();

mkdirSync(assetsDir, { recursive: true });
mkdirSync(dirname(namesFile), { recursive: true });
let changed = 0;
for (const file of files) {
  if (writeIfChanged(join(assetsDir, `${file}.svg`), renderSvg(readNodes(file)))) changed += 1;
}
for (const existing of readdirSync(assetsDir)) {
  if (existing.endsWith(".svg") && !files.includes(existing.slice(0, -4))) {
    unlinkSync(join(assetsDir, existing));
    changed += 1;
  }
}
entries.sort((a, b) => a.name.localeCompare(b.name, "en"));
if (writeIfChanged(namesFile, renderNames(entries))) changed += 1;

console.log(
  `imported names: ${imported.length} (icons ${entries.length}, skipped non-icons ${skipped.length}), ` +
    `svg files: ${files.length}, missing: 0, changed: ${changed}, ` +
    `output: ${relative(repoRoot, assetsDir)}`,
);
