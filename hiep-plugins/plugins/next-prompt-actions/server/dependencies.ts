import { readFileSync, existsSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";

// evaluatorOff: the user disabled jev-evaluator on purpose, so auto-run is off without a warning.
export type Dependencies = { board: boolean; evaluator: boolean; evaluatorOff?: boolean };

export function readDependencies(configFile: string): Dependencies {
  try {
    const config = JSON.parse(readFileSync(configFile, "utf8"));
    const board = config.plugins?.board;
    const jev = config.plugins?.["jev-evaluator"];
    if (jev?.enabled === false)
      return { board: !!board && board.enabled !== false, evaluator: false, evaluatorOff: true };
    const root = jev?.path;
    let evaluator = false;
    if (typeof root === "string" && path.isAbsolute(root)) {
      try {
        const require = createRequire(path.join(root, "package.json"));
        require.resolve("@ai-sdk/gateway");
        evaluator = existsSync(path.join(root, "server/evaluate.mjs"));
      } catch {
        // The worker needs the evaluator's files and SDK, not its running MCP server.
      }
    }
    return { board: !!board && board.enabled !== false, evaluator };
  } catch {
    return { board: false, evaluator: false };
  }
}

export function dependencyWarning({ board, evaluator, evaluatorOff }: Dependencies): string {
  return [
    !board && "Board unavailable. Back to Board will not open; manual Send still works.",
    !evaluator &&
      !evaluatorOff &&
      "Jev evaluator unavailable. Auto-run needs jev-evaluator; manual Send still works.",
  ]
    .filter(Boolean)
    .join(" ");
}
