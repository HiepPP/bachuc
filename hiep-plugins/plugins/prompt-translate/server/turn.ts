import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import type { AgentPromptInput } from "@getpaseo/protocol/agent-types";
import type { TranslateSettings } from "../shared/settings";
import { hasVietnamese } from "../shared/vietnamese";

type Mode = TranslateSettings["cavemanMode"];
type DaemonContext = (input: {
  prompt: string;
  agentId: string;
  cwd: string;
  dir: string;
  runtime: { cavemanRoot: string };
  choice: { mode: Mode; replyVietnamese: boolean; chineseScript: string };
  env?: NodeJS.ProcessEnv;
}) => string;

/** Reads the Caveman install that `scripts/install-hooks.mjs` recorded; null when absent. */
export function readRuntime(dataDir: string): { cavemanRoot: string } | null {
  const file = path.join(dataDir, "hook-runtime.json");
  if (!existsSync(file)) return null;
  const runtime = JSON.parse(readFileSync(file, "utf8"));
  return typeof runtime.cavemanRoot === "string" ? runtime : null;
}

export function promptText(prompt: AgentPromptInput): string {
  if (typeof prompt === "string") return prompt;
  return prompt.map((block) => (block.type === "text" ? block.text : "")).join("\n");
}

export function appendContext(prompt: AgentPromptInput, context: string): AgentPromptInput {
  if (typeof prompt === "string") return `${prompt}\n\n${context}`;
  return [...prompt, { type: "text", text: context }];
}

/** The Caveman and reply-language context the native hook used to add, built in the daemon. */
export function turnContext(input: {
  hookFile: string;
  prompt: string;
  source: string;
  agentId: string;
  cwd: string;
  dir: string;
  runtime: { cavemanRoot: string };
  mode: Mode;
  settings: Pick<TranslateSettings, "matchReplyLanguage" | "chineseScript">;
}): string {
  // The plugin bundle has no import.meta, so the CommonJS hook loads from its absolute path.
  const { daemonContext } = createRequire(input.hookFile)(input.hookFile) as {
    daemonContext: DaemonContext;
  };
  return daemonContext({
    prompt: input.prompt,
    agentId: input.agentId,
    cwd: input.cwd,
    dir: input.dir,
    runtime: input.runtime,
    // Inside the desktop app, process.execPath is the Electron helper; it runs the Caveman
    // scripts as Node only with this flag.
    env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" },
    choice: {
      mode: input.mode,
      replyVietnamese: input.settings.matchReplyLanguage && hasVietnamese(input.source),
      chineseScript: input.settings.chineseScript,
    },
  });
}
