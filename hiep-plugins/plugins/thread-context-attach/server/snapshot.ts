import type { Thread } from "../shared/threads";

export const MAX_REPLY_CHARS = 20_000;
export const NO_REPLY_TEXT = "This thread has no assistant reply yet.";

interface TimelineEntryLike {
  item: { type: string; text?: unknown };
}

/** Returns the text of the newest non-empty assistant message, or null when there is none. */
export function lastAssistantText(entries: readonly TimelineEntryLike[]): string | null {
  for (let index = entries.length - 1; index >= 0; index--) {
    const { item } = entries[index];
    if (item.type === "assistant_message" && typeof item.text === "string" && item.text.trim())
      return item.text;
  }
  return null;
}

export function truncateReply(
  text: string,
  max = MAX_REPLY_CHARS,
): { text: string; truncated: boolean } {
  if (text.length <= max) return { text, truncated: false };
  return {
    text: `${text.slice(0, max)}\n\n[Truncated: showing the first ${max.toLocaleString("en-US")} of ${text.length.toLocaleString("en-US")} characters.]`,
    truncated: true,
  };
}

export function buildSnapshot(
  thread: Pick<Thread, "id" | "title" | "provider" | "cwd">,
  reply: string | null,
  emptyText = NO_REPLY_TEXT,
) {
  const body = reply === null ? { text: emptyText, truncated: false } : truncateReply(reply);
  return {
    agentId: thread.id,
    title: thread.title,
    provider: thread.provider,
    cwd: thread.cwd,
    hasReply: reply !== null,
    truncated: body.truncated,
    text: [
      `# Thread: ${thread.title}`,
      `Provider: ${thread.provider}`,
      `Cwd: ${thread.cwd}`,
      "",
      "## Last reply",
      body.text,
    ].join("\n"),
  };
}

function activityTime(thread: Pick<Thread, "lastActivityAt">): number {
  const time = Date.parse(thread.lastActivityAt);
  return Number.isNaN(time) ? 0 : time;
}

/** Newest first, current workspace first, current agent excluded. */
export function orderThreads(
  threads: readonly Thread[],
  context: { currentAgentId?: string; currentWorkspaceId?: string } = {},
): Thread[] {
  const current = threads.find((thread) => thread.id === context.currentAgentId);
  const workspaceId = context.currentWorkspaceId ?? current?.workspaceId ?? null;
  const inWorkspace = (thread: Thread) =>
    workspaceId
      ? thread.workspaceId === workspaceId
      : current !== undefined && thread.cwd === current.cwd;
  return threads
    .filter((thread) => thread.id !== context.currentAgentId)
    .sort(
      (a, b) =>
        Number(inWorkspace(b)) - Number(inWorkspace(a)) || activityTime(b) - activityTime(a),
    );
}

export function matchesQuery(thread: Thread, query: string): boolean {
  const haystack = `${thread.title} ${thread.provider} ${thread.cwd} ${thread.id}`.toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((word) => haystack.includes(word));
}
