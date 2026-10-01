import path from "node:path";
import type { AgentPromptInput } from "@getpaseo/protocol/agent-types";
import { catalog, type SkillId } from "../shared/catalog";

const reinvokeIds: readonly string[] = catalog
  .filter((skill) => "reinvoke" in skill && skill.reinvoke)
  .map((skill) => skill.id);

const HEADERS = {
  codex: {
    once: "The user pinned these skills for this conversation. Before you respond, read each file and follow it. Skip a file only if its full text is still visible in your current context; an earlier read that has since been summarised away does not count:",
    every:
      "Read and follow these files again on this turn, even if you already read them earlier in this conversation:",
  },
  claude: {
    once: "The user pinned these skills for this conversation. Before you respond, invoke every one of them with the Skill tool. Skip a skill only if its full text is still visible in your current context; an earlier load that has since been summarised away does not count:",
    every:
      "Invoke these skills with the Skill tool on this turn, even if you already invoked them earlier in this conversation:",
  },
};

export type SkillProvider = keyof typeof HEADERS;

const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** An explicit `/id` or `$id` in the prompt already loads that skill. */
export function requested(prompt: string, id: string): boolean {
  return new RegExp(`(?:^|\\s)(?:/(?:[\\w-]+:)?|\\$)${escape(id)}(?![\\w-])`).test(prompt);
}

/** Claude names skills for the Skill tool; Codex reads SKILL.md files under `skillsRoot`. */
export function skillContext(
  skills: readonly SkillId[],
  provider: SkillProvider,
  skillsRoot: string,
): string {
  const headers = HEADERS[provider];
  const entry =
    provider === "codex"
      ? (id: string) => `- ${path.join(skillsRoot, id, "SKILL.md")}`
      : (id: string) => `- ${id}`;
  const block = (group: readonly string[], header: string) =>
    group.length ? [header, ...group.map(entry)] : [];
  return [
    ...block(
      skills.filter((id) => !reinvokeIds.includes(id)),
      headers.once,
    ),
    ...block(
      skills.filter((id) => reinvokeIds.includes(id)),
      headers.every,
    ),
    "Only the skills listed above are pinned now. Stop following any skill that was pinned earlier in this conversation but is missing from this list.",
    "User instructions in the current prompt take priority over pinned skills.",
  ].join("\n");
}

export function promptText(prompt: AgentPromptInput): string {
  if (typeof prompt === "string") return prompt;
  return prompt.map((block) => (block.type === "text" ? block.text : "")).join("\n");
}

export function appendContext(prompt: AgentPromptInput, context: string): AgentPromptInput {
  if (typeof prompt === "string") return `${prompt}\n\n${context}`;
  return [...prompt, { type: "text", text: context }];
}
