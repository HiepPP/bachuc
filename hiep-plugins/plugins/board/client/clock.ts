import { useSyncExternalStore } from "react";

// One shared one-second timer for every time label on the Board.
const listeners = new Set<() => void>();
let timer: ReturnType<typeof setInterval> | undefined;
let tick = Date.now();

function subscribe(listener: () => void) {
  listeners.add(listener);
  timer ??= setInterval(() => {
    tick = Date.now();
    for (const notify of listeners) notify();
  }, 1_000);
  return () => {
    listeners.delete(listener);
    if (listeners.size) return;
    clearInterval(timer);
    timer = undefined;
  };
}

/** Rerenders the caller only when the value derived from the current time changes. */
export function useClock<T extends string | number | boolean | null>(read: (now: number) => T): T {
  return useSyncExternalStore(subscribe, () => read(timer ? tick : Date.now()));
}
