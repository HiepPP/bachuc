import type { AgentPromptInput } from "./agent-sdk-types.js";

// Text a plugin appends to a prompt is wrapped so the timeline can drop it when a provider
// replays its own history, which holds the prompt the provider received.
const OPEN = "<paseo-plugin-context>";
const CLOSE = "</paseo-plugin-context>";
const WRAPPED = /\n*<paseo-plugin-context>\n[\s\S]*?\n<\/paseo-plugin-context>/g;

function wrap(text: string): string {
  return `${OPEN}\n${text.trim()}\n${CLOSE}`;
}

/** Marks what an `agent.prompt` hook appended. A prompt the hook replaced stays as returned. */
export function markPluginPromptContext(
  original: AgentPromptInput,
  next: AgentPromptInput,
): AgentPromptInput {
  if (typeof original === "string" && typeof next === "string") {
    if (next === original || !next.startsWith(original)) return next;
    const added = next.slice(original.length);
    return added.trim() ? `${original}\n\n${wrap(added)}` : original;
  }
  if (Array.isArray(original) && Array.isArray(next) && next.length > original.length) {
    const kept = next.slice(0, original.length);
    if (JSON.stringify(kept) !== JSON.stringify(original)) return next;
    return [
      ...kept,
      ...next
        .slice(original.length)
        .map((block) =>
          block.type === "text" ? { type: "text" as const, text: wrap(block.text) } : block,
        ),
    ];
  }
  return next;
}

export function stripPluginPromptContext(text: string): string {
  return text.includes(OPEN) ? text.replace(WRAPPED, "") : text;
}
