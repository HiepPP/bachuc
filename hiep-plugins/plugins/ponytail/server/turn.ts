import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import type { AgentPromptInput } from "@getpaseo/protocol/agent-types";
import type { PonytailMode } from "../shared/modes";

/** Reminder turns that follow a full ruleset before the daemon sends it again. */
export const RESEND_AFTER = 9;

/** Reads the Ponytail install that `scripts/install-hooks.mjs` recorded; null when absent. */
export function readRuntime(dataDir: string): { ponytailRoot: string } | null {
  const file = path.join(dataDir, "hook-runtime.json");
  if (!existsSync(file)) return null;
  const runtime = JSON.parse(readFileSync(file, "utf8"));
  return typeof runtime.ponytailRoot === "string" ? runtime : null;
}

export function promptText(prompt: AgentPromptInput): string {
  if (typeof prompt === "string") return prompt;
  return prompt.map((block) => (block.type === "text" ? block.text : "")).join("\n");
}

export function appendContext(prompt: AgentPromptInput, context: string): AgentPromptInput {
  if (typeof prompt === "string") return `${prompt}\n\n${context}`;
  return [...prompt, { type: "text", text: context }];
}

/** The mode an explicit `/ponytail` or `$ponytail` command at the start of a prompt sets. */
export function parseCommand(prompt: string): PonytailMode | null {
  const match = /^\s*[/$]ponytail(?:[ \t]+(\S+))?(?=\s|$)/i.exec(prompt);
  if (!match) return null;
  if (!match[1]) return "full";
  const level = match[1].toLowerCase();
  return level === "lite" || level === "full" || level === "ultra" || level === "off"
    ? level
    : null;
}

/**
 * The Ponytail context for one turn: the full ruleset on a mode change and every 10th turn,
 * a one-line reminder between, one stop line when the mode turns off, then nothing.
 */
export function turnContext(input: {
  ponytailRoot: string;
  mode: PonytailMode;
  lastMode: PonytailMode | undefined;
  turnsSinceRules: number;
}): string | null {
  const { mode } = input;
  if (mode === "off") {
    return input.lastMode === "off"
      ? null
      : "Ponytail is off. Stop following earlier Ponytail rules and answer in your normal style.";
  }
  if (input.lastMode !== mode || input.turnsSinceRules >= RESEND_AFTER) {
    return `Ponytail mode: ${mode}.\n\n${rules(input.ponytailRoot, mode)}`;
  }
  return `Ponytail ${mode} mode is still active. Keep following the Ponytail rules.`;
}

function rules(ponytailRoot: string, mode: PonytailMode): string {
  // The plugin bundle has no import.meta, so the CommonJS module loads from its absolute path.
  // In-process: process.execPath is the Electron helper inside the desktop app, so a spawn
  // would need ELECTRON_RUN_AS_NODE=1.
  const file = path.join(ponytailRoot, "hooks/ponytail-instructions.js");
  const { getPonytailInstructions } = createRequire(file)(file) as {
    getPonytailInstructions: (mode: string) => string;
  };
  return getPonytailInstructions(mode);
}
