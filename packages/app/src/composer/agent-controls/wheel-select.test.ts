import { describe, expect, it } from "vitest";
import {
  INITIAL_WHEEL_STEP_STATE,
  WHEEL_IDLE_RESET_MS,
  WHEEL_STEP_THRESHOLD_PX,
  resolveWheelOptionIndex,
  resolveWheelSteps,
} from "./wheel-select";

const BASE = { threshold: WHEEL_STEP_THRESHOLD_PX, idleMs: WHEEL_IDLE_RESET_MS };

describe("resolveWheelSteps", () => {
  it("uses 48 pixels and a 300 ms idle window", () => {
    expect(WHEEL_STEP_THRESHOLD_PX).toBe(48);
    expect(WHEEL_IDLE_RESET_MS).toBe(300);
  });

  it("gives no step for a 40 pixel delta", () => {
    const result = resolveWheelSteps({
      ...BASE,
      state: INITIAL_WHEEL_STEP_STATE,
      deltaY: 40,
      now: 1000,
    });
    expect(result.steps).toBe(0);
    expect(result.state).toEqual({ accumulated: 40, lastEventAt: 1000 });
  });

  it("gives one step for two 40 pixel deltas inside the idle window and keeps the remainder", () => {
    const first = resolveWheelSteps({
      ...BASE,
      state: INITIAL_WHEEL_STEP_STATE,
      deltaY: 40,
      now: 1000,
    });
    const second = resolveWheelSteps({ ...BASE, state: first.state, deltaY: 40, now: 1100 });
    expect(second.steps).toBe(1);
    expect(second.state).toEqual({ accumulated: 32, lastEventAt: 1100 });
  });

  it("gives two steps for a 100 pixel delta", () => {
    const result = resolveWheelSteps({
      ...BASE,
      state: INITIAL_WHEEL_STEP_STATE,
      deltaY: 100,
      now: 1000,
    });
    expect(result.steps).toBe(2);
    expect(result.state.accumulated).toBe(4);
  });

  it("resets the accumulator after an idle gap", () => {
    const first = resolveWheelSteps({
      ...BASE,
      state: INITIAL_WHEEL_STEP_STATE,
      deltaY: 40,
      now: 1000,
    });
    const second = resolveWheelSteps({
      ...BASE,
      state: first.state,
      deltaY: 40,
      now: 1000 + WHEEL_IDLE_RESET_MS,
    });
    expect(second.steps).toBe(0);
    expect(second.state.accumulated).toBe(40);
  });

  it("gives negative steps for a negative delta", () => {
    const result = resolveWheelSteps({
      ...BASE,
      state: INITIAL_WHEEL_STEP_STATE,
      deltaY: -100,
      now: 1000,
    });
    expect(result.steps).toBe(-2);
    expect(result.state.accumulated).toBe(-4);
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
