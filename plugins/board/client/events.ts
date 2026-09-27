declare const document:
  | {
      addEventListener(type: string, listener: (event: Event) => void): void;
      removeEventListener(type: string, listener: (event: Event) => void): void;
    }
  | undefined;

const subscribers = new Set<(sent: boolean) => void>();
let missedFailureAt = 0;
const MISSED_FAILURE_MS = 5_000;

// Other plugins cannot open this plugin's surface. next-prompt-actions opens the Board before
// its send is acknowledged, then reports the outcome so the Board can refresh or warn.
export function installBoardEvents(
  open: () => void,
  host: () => string | undefined | Promise<string | undefined>,
) {
  if (typeof document === "undefined") return () => {};
  const target = document;
  let disposed = false;
  let eventHost: string | undefined;
  let pending: Promise<string | undefined> | undefined;
  const resolveHost = () => {
    if (eventHost !== undefined) return eventHost;
    if (pending) return pending;
    try {
      const result = host();
      if (typeof result === "string" || result === undefined) {
        eventHost = result;
        return result;
      }
      pending = result
        .then((serverId) => (eventHost = serverId))
        .catch(() => undefined)
        .finally(() => {
          pending = undefined;
        });
      return pending;
    } catch {
      return undefined;
    }
  };
  // Warm the cache; a failed lookup can be retried by the next scoped event.
  void resolveHost();
  const report = (sent: boolean) => {
    // A fast failure can land before the Board mounts; keep it briefly for the next mount.
    if (!subscribers.size && !sent) missedFailureAt = Date.now();
    for (const subscriber of subscribers) subscriber(sent);
  };
  const listeners: [string, (event: Event) => void][] = [
    ["paseo-board:v2:open", open],
    ["paseo-board:v2:sent", () => report(true)],
    ["paseo-board:v2:send-failed", () => report(false)],
  ];
  const scoped = listeners.map(
    ([type, listener]) =>
      [
        type,
        (event: Event) => {
          const serverId = (event as CustomEvent<{ serverId?: unknown }>).detail?.serverId;
          if (typeof serverId !== "string") return;
          const deliver = (resolvedHost: string | undefined) => {
            if (!disposed && serverId === resolvedHost) listener(event);
          };
          const resolvedHost = resolveHost();
          if (typeof resolvedHost === "string" || resolvedHost === undefined) {
            deliver(resolvedHost);
          } else {
            void resolvedHost.then(deliver);
          }
        },
      ] as const,
  );
  for (const [type, listener] of scoped) target.addEventListener(type, listener);
  return () => {
    disposed = true;
    for (const [type, listener] of scoped) target.removeEventListener(type, listener);
  };
}

export function subscribeSendResult(subscriber: (sent: boolean) => void, now = Date.now()) {
  if (now - missedFailureAt < MISSED_FAILURE_MS) subscriber(false);
  missedFailureAt = 0;
  subscribers.add(subscriber);
  return () => {
    subscribers.delete(subscriber);
  };
}
