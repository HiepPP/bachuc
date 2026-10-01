import type { StreamItem } from "@/types/stream";

// An opened agent that never becomes ready must not jump when the reader opens it later.
const REQUEST_TTL_MS = 5000;

let pending: { agentId: string; expiresAt: number } | null = null;
let version = 0;
const listeners = new Set<() => void>();

/**
 * Asks the transcript of `agentId` to open on its latest prompt instead of its bottom. The
 * transcript is already retained or not mounted yet, so the request waits here until it is ready.
 */
export function requestLatestPromptAnchor(agentId: string, now = Date.now()): void {
  pending = { agentId, expiresAt: now + REQUEST_TTL_MS };
  version += 1;
  for (const listener of listeners) listener();
}

/** True once per request, for the agent it names, until it expires. */
export function takeLatestPromptAnchor(agentId: string, now = Date.now()): boolean {
  if (pending?.agentId !== agentId) return false;
  const live = now <= pending.expiresAt;
  pending = null;
  return live;
}

export function hasPendingLatestPromptAnchor(agentId: string, now = Date.now()): boolean {
  return pending?.agentId === agentId && now <= pending.expiresAt;
}

/** Restarts the expiry of a request that waited on a history fetch. */
export function extendLatestPromptAnchor(agentId: string, now = Date.now()): void {
  if (pending?.agentId === agentId) pending.expiresAt = now + REQUEST_TTL_MS;
}

export function subscribeLatestPromptAnchor(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getLatestPromptAnchorVersion(): number {
  return version;
}

export function findLatestPromptId(segments: readonly (readonly StreamItem[])[]): string | null {
  for (let segment = segments.length - 1; segment >= 0; segment -= 1) {
    const items = segments[segment];
    for (let index = items.length - 1; index >= 0; index -= 1) {
      if (items[index].kind === "user_message") return items[index].id;
    }
  }
  return null;
}
