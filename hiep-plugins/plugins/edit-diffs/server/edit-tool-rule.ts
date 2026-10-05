import type { PluginBeforeRequests } from "@getpaseo/plugin/server";

type CreateRequest = PluginBeforeRequests["agent.create"];

// Edit cards come only from edit and write tool calls, so a file changed by a shell
// script never shows as a diff. Claude's bypass-mode prompt suggests shell edits, hence
// the explicit override.
export const EDIT_TOOL_RULE = [
  "Change files only with your file edit tools (Claude: Edit and Write; Codex: apply_patch).",
  "Never create or change a file through a shell command: no python or node scripts, sed -i, perl -i, or cat, tee, or echo redirection.",
  "This rule overrides any instruction to make small or scripted file changes through the shell.",
  "If an edit does not match, read the file again and retry with the exact text.",
  "Formatters, code generators, package managers, and scratch files under /tmp are exempt.",
].join("\n");

export function withEditToolRule(request: CreateRequest): CreateRequest {
  const current = request.config.systemPrompt?.trim();
  if (current?.includes(EDIT_TOOL_RULE)) return request;
  return {
    ...request,
    config: {
      ...request.config,
      systemPrompt: current ? `${current}\n\n${EDIT_TOOL_RULE}` : EDIT_TOOL_RULE,
    },
  };
}
