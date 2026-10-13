export const WHEEL_STEP_THRESHOLD_PX = 40;
// A gap this long ends a wheel gesture. The next event starts a new one.
export const WHEEL_GESTURE_GAP_MS = 150;

export interface WheelStepState {
  accumulated: number;
  lastEventAt: number;
}

export const INITIAL_WHEEL_STEP_STATE: WheelStepState = {
  accumulated: 0,
  lastEventAt: Number.NEGATIVE_INFINITY,
};

/**
 * The first event of a gesture steps once at any size, so one slow notch is one step on any
 * mouse. Later events step once per `threshold` pixels, several at a time on a fast spin.
 */
export function resolveWheelSteps({
  state,
  deltaY,
  now,
  threshold = WHEEL_STEP_THRESHOLD_PX,
  gapMs = WHEEL_GESTURE_GAP_MS,
}: {
  state: WheelStepState;
  deltaY: number;
  now: number;
  threshold?: number;
  gapMs?: number;
}): { steps: number; state: WheelStepState } {
  if (now - state.lastEventAt >= gapMs) {
    return { steps: Math.sign(deltaY), state: { accumulated: 0, lastEventAt: now } };
  }
  const total = state.accumulated + deltaY;
  const steps = Math.trunc(total / threshold);
  return { steps, state: { accumulated: total - steps * threshold, lastEventAt: now } };
}

export function resolveWheelOptionIndex({
  currentIndex,
  steps,
  count,
}: {
  currentIndex: number;
  steps: number;
  count: number;
}): number {
  if (count <= 0) return -1;
  const start = Math.max(currentIndex, 0);
  return Math.min(Math.max(start + steps, 0), count - 1);
}
