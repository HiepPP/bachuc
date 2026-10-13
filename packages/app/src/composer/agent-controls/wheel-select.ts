export const WHEEL_STEP_THRESHOLD_PX = 48;
export const WHEEL_IDLE_RESET_MS = 300;

export interface WheelStepState {
  accumulated: number;
  lastEventAt: number;
}

export const INITIAL_WHEEL_STEP_STATE: WheelStepState = {
  accumulated: 0,
  lastEventAt: Number.NEGATIVE_INFINITY,
};

export function resolveWheelSteps({
  state,
  deltaY,
  now,
  threshold = WHEEL_STEP_THRESHOLD_PX,
  idleMs = WHEEL_IDLE_RESET_MS,
}: {
  state: WheelStepState;
  deltaY: number;
  now: number;
  threshold?: number;
  idleMs?: number;
}): { steps: number; state: WheelStepState } {
  const base = now - state.lastEventAt >= idleMs ? 0 : state.accumulated;
  const total = base + deltaY;
  const steps = Math.trunc(total / threshold);
  return {
    steps,
    state: { accumulated: total - steps * threshold, lastEventAt: now },
  };
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
