import { FRAME_HEIGHT_MESSAGE_KEY } from "@/file-pane/html-preview-csp";

export const AUTO_INITIAL_HEIGHT = 120;
export const AUTO_MIN_HEIGHT = 40;
export const AUTO_MAX_HEIGHT = 8000;
// Equal growth steps in a row that mark a page whose height follows the frame.
const LOOP_STEPS = 3;

export interface AutoHeight {
  height: number;
  lastStep: number;
  equalSteps: number;
  frozen: boolean;
}

export const INITIAL_AUTO_HEIGHT: AutoHeight = {
  height: AUTO_INITIAL_HEIGHT,
  lastStep: 0,
  equalSteps: 0,
  frozen: false,
};

/** The height a frame's page reported: a message object on web, a JSON string on native. */
export function readReportedHeight(data: unknown): number | null {
  let value = data;
  if (typeof value === "string") {
    try {
      value = JSON.parse(value);
    } catch {
      return null;
    }
  }
  if (!value || typeof value !== "object") return null;
  const height = (value as Record<string, unknown>)[FRAME_HEIGHT_MESSAGE_KEY];
  return typeof height === "number" && Number.isFinite(height) ? height : null;
}

// Enough for every frame in the open threads; the oldest entry goes first.
const REMEMBERED_LIMIT = 200;
const remembered = new Map<string, Pick<AutoHeight, "height" | "frozen">>();

/**
 * A short key for a document. The cache holds keys, not documents, so an unmounted frame does
 * not keep its page alive.
 */
export function frameHeightKey(document: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < document.length; index += 1) {
    hash ^= document.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return `${document.length}:${(hash >>> 0).toString(36)}`;
}

/**
 * The size a frame starts with. A page shown before starts at its last height, so a remount,
 * such as scrolling back to the row or reopening the thread, reserves its box before the first
 * paint instead of jumping from the default.
 */
export function initialAutoHeight(key: string): AutoHeight {
  const last = remembered.get(key);
  return last ? { ...INITIAL_AUTO_HEIGHT, ...last } : INITIAL_AUTO_HEIGHT;
}

/** Remembers a frame's settled size for the next mount of the same page. */
export function rememberAutoHeight(key: string, state: AutoHeight): void {
  remembered.delete(key);
  remembered.set(key, { height: state.height, frozen: state.frozen });
  if (remembered.size > REMEMBERED_LIMIT) {
    const oldest = remembered.keys().next().value;
    if (oldest !== undefined) remembered.delete(oldest);
  }
}

/**
 * Sizes the frame to the page's content, clamped because the page is untrusted. A page sized
 * with `100vh` or `height: 100%` grows by the same step on every report; after three equal
 * steps the frame stops growing and that page scrolls inside it.
 */
export function nextAutoHeight(state: AutoHeight, reported: number): AutoHeight {
  if (state.frozen) return state;
  const height = Math.min(AUTO_MAX_HEIGHT, Math.max(AUTO_MIN_HEIGHT, Math.ceil(reported)));
  const step = height - state.height;
  if (Math.abs(step) < 1) return state;
  const equalSteps = step > 0 && step === state.lastStep ? state.equalSteps + 1 : 1;
  if (equalSteps >= LOOP_STEPS) return { ...state, frozen: true };
  return { height, lastStep: step, equalSteps, frozen: false };
}
