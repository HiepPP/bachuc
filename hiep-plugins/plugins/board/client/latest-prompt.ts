export interface PromptTarget {
  /** Latest prompt's top minus the transcript viewport's top, in px. */
  offset: number;
  /** Pixels the transcript can still scroll down. */
  room: number;
  scrollBy(delta: number): void;
}

export interface RevealEnv {
  now(): number;
  frame(callback: () => void): number;
  cancelFrame(id: number): void;
  find(): PromptTarget | null;
  /** DOM mutations and scrolls; both are delivered before the browser paints them. */
  onChange(listener: () => void): () => void;
  onUserInput(listener: () => void): () => void;
}

// Matches Paseo's own prompt jump inset.
export const PROMPT_TOP_INSET_PX = 8;
const TOLERANCE_PX = 1;
const GIVE_UP_MS = 5000;
// Paseo keeps re-pinning to the bottom while the opened thread loads and activates.
const HOLD_MS = 1500;

/**
 * Paseo pins an opened thread to its bottom and `openAgent` has no scroll anchor, so this
 * holds the latest prompt at the top while the thread opens. It corrects on every DOM
 * mutation and scroll, before the browser paints, so the bottom is never shown.
 * Any real scroll, key, pointer, or touch input from the user cancels it.
 */
export function revealLatestPrompt(env: RevealEnv): () => void {
  const startedAt = env.now();
  let foundAt: number | null = null;
  let frame = 0;
  let stopped = false;
  let cleanups: Array<() => void> = [];
  const stop = () => {
    if (stopped) return;
    stopped = true;
    env.cancelFrame(frame);
    for (const cleanup of cleanups) cleanup();
  };
  const check = () => {
    if (stopped) return;
    const now = env.now();
    if (foundAt === null ? now - startedAt > GIVE_UP_MS : now - foundAt > HOLD_MS) return stop();
    const target = env.find();
    if (!target) return;
    foundAt ??= now;
    const delta = target.offset - PROMPT_TOP_INSET_PX;
    if (delta < -TOLERANCE_PX) target.scrollBy(delta);
    // Rows above the prompt can grow after it is placed; a short thread has no room to move.
    else if (delta > TOLERANCE_PX && target.room > TOLERANCE_PX)
      target.scrollBy(Math.min(delta, target.room));
  };
  const tick = () => {
    check();
    if (!stopped) frame = env.frame(tick);
  };
  cleanups = [env.onUserInput(stop), env.onChange(check)];
  tick();
  return stop;
}

interface DomRect {
  top: number;
}
interface DomElement {
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
  getBoundingClientRect(): DomRect;
  getClientRects(): { length: number };
  closest(selector: string): DomElement | null;
  querySelectorAll(selector: string): Iterable<DomElement>;
  dispatchEvent(event: unknown): boolean;
}
interface InputEvent {
  isTrusted: boolean;
}
type InputListener = (event: InputEvent) => void;
declare const document: {
  body: unknown;
  querySelectorAll(selector: string): Iterable<DomElement>;
  addEventListener(type: string, listener: () => void, options: object): void;
  removeEventListener(type: string, listener: () => void, options: object): void;
};
declare const window: {
  requestAnimationFrame(callback: () => void): number;
  cancelAnimationFrame(id: number): void;
  addEventListener(type: string, listener: InputListener, options: object): void;
  removeEventListener(type: string, listener: InputListener, options: object): void;
};
declare const WheelEvent: new (type: string, init: { deltaY: number; bubbles: boolean }) => unknown;
declare const MutationObserver: new (callback: () => void) => {
  observe(target: unknown, options: { childList: boolean; subtree: boolean }): void;
  disconnect(): void;
};

const USER_INPUT_EVENTS = ["wheel", "keydown", "pointerdown", "touchstart"];

function findLatestPrompt(): PromptTarget | null {
  const container = [...document.querySelectorAll('[data-testid="agent-chat-scroll"]')]
    .filter((element) => element.clientHeight > 0 && element.getClientRects().length > 0)
    .at(-1);
  if (!container) return null;
  // Paseo's mounted recent window always starts at a user message, so the latest prompt
  // is in the DOM even when older history is virtualized.
  const message = [...container.querySelectorAll('[data-testid="user-message"]')].at(-1);
  if (!message) return null;
  const row = message.closest("[data-history-row-id]") ?? message;
  return {
    offset: row.getBoundingClientRect().top - container.getBoundingClientRect().top,
    room: container.scrollHeight - container.clientHeight - container.scrollTop,
    scrollBy(delta) {
      // Paseo keeps following output unless an upward wheel precedes the scroll.
      if (delta < 0)
        container.dispatchEvent(new WheelEvent("wheel", { deltaY: -1, bubbles: true }));
      container.scrollTop += delta;
    },
  };
}

const browserEnv: RevealEnv = {
  now: () => performance.now(),
  frame: (callback) => window.requestAnimationFrame(callback),
  cancelFrame: (id) => window.cancelAnimationFrame(id),
  find: findLatestPrompt,
  onChange(listener) {
    const observer = new MutationObserver(listener);
    observer.observe(document.body, { childList: true, subtree: true });
    // Element scroll events do not bubble, but capture listeners still see them.
    document.addEventListener("scroll", listener, { capture: true, passive: true });
    return () => {
      observer.disconnect();
      document.removeEventListener("scroll", listener, { capture: true });
    };
  },
  onUserInput(listener) {
    const handle: InputListener = (event) => {
      if (event.isTrusted) listener();
    };
    for (const type of USER_INPUT_EVENTS) {
      window.addEventListener(type, handle, { capture: true, passive: true });
    }
    return () => {
      for (const type of USER_INPUT_EVENTS) {
        window.removeEventListener(type, handle, { capture: true });
      }
    };
  },
};

let cancelReveal = () => {};

/** Web/desktop only: native clients render the transcript without a DOM. */
export function revealLatestPromptOnWeb() {
  if (typeof document === "undefined" || typeof window === "undefined") return;
  cancelReveal();
  cancelReveal = revealLatestPrompt(browserEnv);
}
