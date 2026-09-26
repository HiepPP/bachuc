import type { PaseoApi } from "@getpaseo/client";

// Prompt Translate prepends `$caveman <mode>` to a new thread's first prompt, and Paseo titles
// a thread with the first prompt line, so the title names the mode instead of the goal.
const COMMAND_LINE = /^[/$]caveman(?::caveman)?(?:\s+\S+)?$/i;
// Matches Paseo's provisional title length.
const MAX_TITLE_CHARS = 60;

const normalize = (line: string) => line.replace(/\s+/g, " ").trim();

export function isCommandTitle(title: string) {
  return COMMAND_LINE.test(normalize(title));
}

export function titleFromPrompt(prompt: string): string | null {
  const line = prompt
    .split(/\r?\n/)
    .map(normalize)
    .find((text) => text && !COMMAND_LINE.test(text));
  return line ? line.slice(0, MAX_TITLE_CHARS).trim() : null;
}

/** Reads the goal from the prompt that produced `title`; null when that prompt is gone. */
export async function firstPromptTitle(
  paseo: Pick<PaseoApi, "agents">,
  agentId: string,
  title: string,
): Promise<string | null> {
  const page = await paseo.agents.ref(agentId).timeline.refetch({ direction: "after", limit: 20 });
  const prompt = page.entries.find(({ item }) => item.type === "user_message")?.item;
  if (prompt?.type !== "user_message") return null;
  const first = prompt.text.split(/\r?\n/).map(normalize).find(Boolean);
  return first === normalize(title) ? titleFromPrompt(prompt.text) : null;
}
