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
export function installBoardEvents(open: () => void, host: () => string | undefined) {
  if (typeof document === "undefined") return () => {};
  const target = document;
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
          if (typeof serverId === "string" && serverId === host()) listener(event);
        },
      ] as const,
  );
  for (const [type, listener] of scoped) target.addEventListener(type, listener);
  return () => {
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
