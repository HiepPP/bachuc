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
