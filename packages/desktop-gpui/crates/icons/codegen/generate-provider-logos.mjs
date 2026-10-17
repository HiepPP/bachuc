#!/usr/bin/env node
// Writes one filled SVG template per Electron provider logo into `assets/providers/`.
// The glyphs come from the React components Electron draws (packages/app/src/components/icons),
// except `glm`, which Electron draws from its bundled acp-provider-icons SVG.
// Each template holds `{FILL}` and `{OPACITY}` placeholders that `provider_logo` fills.
// Usage: node generate-provider-logos.mjs
// Output is deterministic, so a second run changes nothing.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const crateDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = join(crateDir, "..", "..", "..", "..");
const appSrc = join(repoRoot, "packages", "app", "src");
const outDir = join(crateDir, "assets", "providers");

const COMPONENTS = ["claude", "codex", "copilot", "minimax", "omp", "opencode", "pi"];
const GLM_SVG = join(appSrc, "assets", "acp-provider-icons", "glm-acp-agent.svg");

const kebab = (name) => name.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();

function attributes(text) {
  const found = new Map();
  for (const match of text.matchAll(/([A-Za-z-]+)=(?:"([^"]*)"|\{([^}]*)\})/g)) {
    found.set(match[1], match[2] ?? match[3]);
  }
  return found;
}

// Electron paints with the `color` prop; `{FILL}` takes its place.
function renderPath(attrs) {
  const rendered = [...attrs]
    .filter(([key, value]) => key !== "fill" || value !== "color")
    .map(([key, value]) => `${kebab(key)}="${value}"`);
  return `  <path ${rendered.join(" ")}/>`;
}

function render(viewBox, rootAttrs, paths) {
  const root = ['xmlns="http://www.w3.org/2000/svg"', 'width="24"', 'height="24"'];
  root.push(`viewBox="${viewBox}"`, 'fill="{FILL}"', 'fill-opacity="{OPACITY}"');
  if (rootAttrs.has("fillRule")) root.push(`fill-rule="${rootAttrs.get("fillRule")}"`);
  return [`<svg ${root.join(" ")}>`, ...paths.map(renderPath), "</svg>", ""].join("\n");
}

function fromComponent(name) {
  const source = readFileSync(join(appSrc, "components", "icons", `${name}-icon.tsx`), "utf8");
  const svg = source.match(/<Svg\b([^>]*)>/);
  if (!svg) throw new Error(`no <Svg> in ${name}-icon.tsx`);
  const rootAttrs = attributes(svg[1]);
  const paths = [...source.matchAll(/<Path\b([^>]*?)\/>/gs)].map((match) => attributes(match[1]));
  if (paths.length === 0 || paths.some((attrs) => !attrs.has("d"))) {
    throw new Error(`cannot read the paths of ${name}-icon.tsx`);
  }
  return render(rootAttrs.get("viewBox"), rootAttrs, paths);
}

function fromSvgFile(file) {
  const source = readFileSync(file, "utf8");
  const svg = source.match(/<svg\b([^>]*)>/);
  if (!svg) throw new Error(`no <svg> in ${file}`);
  const rootAttrs = attributes(svg[1]);
  rootAttrs.set("fillRule", rootAttrs.get("fill-rule"));
  const paths = [...source.matchAll(/<path\b([^>]*?)\/>/gs)].map((match) => attributes(match[1]));
  if (paths.length === 0) throw new Error(`no <path> in ${file}`);
  return render(rootAttrs.get("viewBox"), rootAttrs, paths);
}

function writeIfChanged(path, content) {
  if (existsSync(path) && readFileSync(path, "utf8") === content) return false;
  writeFileSync(path, content);
  return true;
}

mkdirSync(outDir, { recursive: true });
const logos = new Map(COMPONENTS.map((name) => [name, fromComponent(name)]));
logos.set("glm", fromSvgFile(GLM_SVG));

let changed = 0;
for (const [name, svg] of logos) {
  if (writeIfChanged(join(outDir, `${name}.svg`), svg)) changed += 1;
}

console.log(
  `provider logos: ${logos.size}, changed: ${changed}, output: ${relative(repoRoot, outDir)}`,
);
