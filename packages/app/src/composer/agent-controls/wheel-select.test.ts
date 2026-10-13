import { describe, expect, it } from "vitest";
import {
  INITIAL_WHEEL_STEP_STATE,
  WHEEL_GESTURE_GAP_MS,
  WHEEL_STEP_THRESHOLD_PX,
  resolveWheelOptionIndex,
  resolveWheelSteps,
} from "./wheel-select";

function startGesture(deltaY: number) {
  return resolveWheelSteps({ state: INITIAL_WHEEL_STEP_STATE, deltaY, now: 1000 });
}

describe("resolveWheelSteps", () => {
  it("uses 40 pixels per step and a 150 ms gap between gestures", () => {
    expect(WHEEL_STEP_THRESHOLD_PX).toBe(40);
    expect(WHEEL_GESTURE_GAP_MS).toBe(150);
  });

  it("steps once on the first event of a gesture, however small", () => {
    expect(startGesture(4).steps).toBe(1);
    expect(startGesture(-4).steps).toBe(-1);
    expect(startGesture(300).steps).toBe(1);
    expect(startGesture(4).state.accumulated).toBe(0);
  });

  it("steps once per 40 pixels inside a gesture and keeps the rest", () => {
    const first = startGesture(4);
    const second = resolveWheelSteps({ state: first.state, deltaY: 30, now: 1020 });
    expect(second.steps).toBe(0);
    const third = resolveWheelSteps({ state: second.state, deltaY: 20, now: 1040 });
    expect(third.steps).toBe(1);
    expect(third.state).toEqual({ accumulated: 10, lastEventAt: 1040 });
  });

  it("gives several steps for one large delta inside a gesture", () => {
    const first = startGesture(10);
    const fast = resolveWheelSteps({ state: first.state, deltaY: 130, now: 1030 });
    expect(fast.steps).toBe(3);
    const back = resolveWheelSteps({ state: fast.state, deltaY: -130, now: 1060 });
    expect(back.steps).toBe(-3);
  });

  it("starts a new gesture after the gap", () => {
    const first = startGesture(4);
    const next = resolveWheelSteps({
      state: first.state,
      deltaY: 4,
      now: 1000 + WHEEL_GESTURE_GAP_MS,
    });
    expect(next.steps).toBe(1);
  });
});

describe("resolveWheelOptionIndex", () => {
  it("moves by the signed step count", () => {
    expect(resolveWheelOptionIndex({ currentIndex: 1, steps: 1, count: 4 })).toBe(2);
    expect(resolveWheelOptionIndex({ currentIndex: 2, steps: -1, count: 4 })).toBe(1);
  });

  it("clamps at the last option", () => {
    expect(resolveWheelOptionIndex({ currentIndex: 3, steps: 1, count: 4 })).toBe(3);
    expect(resolveWheelOptionIndex({ currentIndex: 2, steps: 5, count: 4 })).toBe(3);
  });

  it("clamps at the first option", () => {
    expect(resolveWheelOptionIndex({ currentIndex: 0, steps: -1, count: 4 })).toBe(0);
    expect(resolveWheelOptionIndex({ currentIndex: 1, steps: -5, count: 4 })).toBe(0);
  });

  it("treats an unknown current index as the first option", () => {
    expect(resolveWheelOptionIndex({ currentIndex: -1, steps: 1, count: 4 })).toBe(1);
    expect(resolveWheelOptionIndex({ currentIndex: -1, steps: -1, count: 4 })).toBe(0);
  });

  it("returns -1 for an empty list", () => {
    expect(resolveWheelOptionIndex({ currentIndex: 0, steps: 1, count: 0 })).toBe(-1);
  });
});
