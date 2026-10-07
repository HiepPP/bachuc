export interface TimelinePassageRequest {
  messageId: string;
  text?: string;
}

export type TimelinePassageRevealer = (request: TimelinePassageRequest) => void;

// Long enough for a navigation to mount the agent's stream, short enough that a stale request
// never fires when the user opens that agent much later.
const PENDING_TTL_MS = 10_000;

const revealers = new Map<string, TimelinePassageRevealer[]>();
const pending = new Map<string, { request: TimelinePassageRequest; expiresAt: number }>();

function revealerKey(serverId: string, agentId: string): string {
  return `${serverId}\u0000${agentId}`;
}

/**
 * A mounted agent stream registers how it reveals a passage. The latest mount handles requests,
 * and it takes over a request made just before it mounted.
 */
export function registerTimelinePassageRevealer(
  serverId: string,
  agentId: string,
  revealer: TimelinePassageRevealer,
): () => void {
  const key = revealerKey(serverId, agentId);
  revealers.set(key, [...(revealers.get(key) ?? []), revealer]);
  const waiting = pending.get(key);
  if (waiting) {
    pending.delete(key);
    if (waiting.expiresAt > Date.now()) revealer(waiting.request);
  }
  return () => {
    const rest = (revealers.get(key) ?? []).filter((candidate) => candidate !== revealer);
    if (rest.length > 0) revealers.set(key, rest);
    else revealers.delete(key);
  };
}

/** Reveals at once in a mounted stream, or holds the request until the stream mounts. */
export function requestTimelinePassage(
  serverId: string,
  agentId: string,
  request: TimelinePassageRequest,
): void {
  const key = revealerKey(serverId, agentId);
  const revealer = revealers.get(key)?.at(-1);
  if (revealer) {
    revealer(request);
    return;
  }
  pending.set(key, { request, expiresAt: Date.now() + PENDING_TTL_MS });
}
