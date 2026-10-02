#!/usr/bin/env node
// Stages every plugin into hiep-plugins/.bundle/<id> for packages/desktop/electron-builder.yml,
// which copies that folder into the app as Resources/plugins. The daemon compiles plugin source at
// load time and some plugins spawn tsx workers, so a staged plugin keeps its source and gets its
// production node_modules; tests, benchmarks, and dev dependencies stay out.
import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourceRoot = path.join(root, "plugins");
const outputRoot = path.join(root, ".bundle");
const SKIPPED = new Set(["node_modules", "tests", "benchmark"]);

// Start clean so a plugin removed from plugins/ does not stay in the app.
rmSync(outputRoot, { recursive: true, force: true });
mkdirSync(outputRoot, { recursive: true });

const staged = [];
for (const entry of readdirSync(sourceRoot, { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  const source = path.join(sourceRoot, entry.name);
  if (!existsSync(path.join(source, "paseo-plugin.json"))) continue;
  const output = path.join(outputRoot, entry.name);
  cpSync(source, output, {
    recursive: true,
    filter: (file) => !SKIPPED.has(path.basename(file)),
  });
  const manifest = JSON.parse(readFileSync(path.join(source, "package.json"), "utf8"));
  const dependencies = Object.keys(manifest.dependencies ?? {});
  if (dependencies.length > 0) {
    // Install scripts are skipped: plugins are trusted, their dependencies' scripts are not needed.
    execFileSync(
      "npm",
      ["ci", "--omit=dev", "--ignore-scripts", "--no-audit", "--no-fund", "--loglevel=error"],
      { cwd: output, stdio: "inherit" },
    );
  }
  staged.push(`${entry.name}${dependencies.length > 0 ? ` (${dependencies.join(", ")})` : ""}`);
}

if (staged.length === 0) throw new Error(`No plugins found in ${sourceRoot}`);
console.log(`Staged ${staged.length} plugins into ${outputRoot}:\n  ${staged.join("\n  ")}`);
