import type { StreamItem } from "@/types/stream";

// A request ends when its jump lands or the reader scrolls, when another open replaces it, or
// when the timeline has no prompt. The cap only covers an agent that never becomes ready.
const REQUEST_CAP_MS = 30_000;

let pending: { agentId: string; requestId: number; expiresAt: number; begun: boolean } | null =
  null;
let capTimer: ReturnType<typeof setTimeout> | null = null;
let nextRequestId = 1;
let version = 0;
const listeners = new Set<() => void>();

function notify(): void {
  version += 1;
  for (const listener of listeners) listener();
}

function clearPending(): void {
  pending = null;
  if (capTimer !== null) clearTimeout(capTimer);
  capTimer = null;
}

/**
 * Asks the transcript of `agentId` to open on its latest prompt instead of its bottom. The
 * transcript is already retained or not mounted yet, so the request waits here until it lands.
 */
export function requestLatestPromptAnchor(agentId: string, now = Date.now()): void {
  clearPending();
  const request = {
    agentId,
    requestId: nextRequestId++,
    expiresAt: now + REQUEST_CAP_MS,
    begun: false,
  };
  pending = request;
  capTimer = setTimeout(() => {
    if (pending !== request) return;
    clearPending();
    notify();
  }, REQUEST_CAP_MS);
  notify();
}

/** True from the request until it lands, is cancelled, or reaches the cap. */
export function hasPendingLatestPromptAnchor(agentId: string, now = Date.now()): boolean {
  return pending?.agentId === agentId && now <= pending.expiresAt;
}

/**
 * Marks the jump of the pending request as started, once per request. The request stays
 * pending until `settleLatestPromptAnchor`. Returns the request id, or null when there is
 * nothing to start.
 */
export function beginLatestPromptAnchor(agentId: string, now = Date.now()): number | null {
  if (!pending || !hasPendingLatestPromptAnchor(agentId, now) || pending.begun) return null;
  pending.begun = true;
  return pending.requestId;
}

/**
 * Ends the pending request of `agentId`. With `requestId`, ends only that request, so a jump
 * that a newer request replaced does not end the newer one.
 */
export function settleLatestPromptAnchor(agentId: string, requestId?: number): void {
  if (pending?.agentId !== agentId) return;
  if (requestId !== undefined && pending.requestId !== requestId) return;
  clearPending();
  notify();
}

/** Ends any pending request, so a bottom open is not taken over by an older prompt jump. */
export function cancelLatestPromptAnchor(): void {
  if (!pending) return;
  clearPending();
  notify();
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
