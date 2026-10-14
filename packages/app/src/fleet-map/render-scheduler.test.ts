import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createRenderScheduler, type RenderScheduler } from "./render-scheduler";

// A 60 Hz display: the fake frame source fires every 1000/60 ms of fake time.
const FRAME_MS = 1000 / 60;

function createHarness() {
  const render = vi.fn();
  const scheduler = createRenderScheduler({
    requestFrame: (callback) => setTimeout(callback, FRAME_MS) as unknown as number,
    cancelFrame: (handle) => clearTimeout(handle),
    now: () => Date.now(),
    render,
  });
  return { render, scheduler };
}

function startPulse(scheduler: RenderScheduler) {
  scheduler.setVisible(true);
  scheduler.setFocused(true);
  scheduler.setReducedMotion(false);
  scheduler.setPulsing(true);
}

describe("createRenderScheduler", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders 0 frames in 2 seconds with no invalidate and no pulse", () => {
    const { render } = createHarness();
    vi.advanceTimersByTime(2000);
    expect(render).toHaveBeenCalledTimes(0);
  });

  it("renders 1 frame for many invalidate calls in one tick", () => {
    const { render, scheduler } = createHarness();
    for (let i = 0; i < 50; i += 1) {
      scheduler.invalidate();
    }
    vi.advanceTimersByTime(1000);
    expect(render).toHaveBeenCalledTimes(1);
  });

  it("renders again after a later invalidate", () => {
    const { render, scheduler } = createHarness();
    scheduler.invalidate();
    vi.advanceTimersByTime(500);
    scheduler.invalidate();
    vi.advanceTimersByTime(500);
    expect(render).toHaveBeenCalledTimes(2);
  });

  it("pulses at most 30 frames per second when every pulse flag allows it", () => {
    const { render, scheduler } = createHarness();
    startPulse(scheduler);
    vi.advanceTimersByTime(1000);
    expect(render.mock.calls.length).toBeGreaterThan(0);
    expect(render.mock.calls.length).toBeLessThanOrEqual(30);
  });

  it("keeps the 30 frames per second cap while invalidate is called every frame", () => {
    const { render, scheduler } = createHarness();
    startPulse(scheduler);
    for (let i = 0; i < 60; i += 1) {
      scheduler.invalidate();
      vi.advanceTimersByTime(FRAME_MS);
    }
    expect(render.mock.calls.length).toBeLessThanOrEqual(30);
  });

  it.each([
    ["not pulsing", (s: RenderScheduler) => s.setPulsing(false)],
    ["not visible", (s: RenderScheduler) => s.setVisible(false)],
    ["not focused", (s: RenderScheduler) => s.setFocused(false)],
    ["reduced motion", (s: RenderScheduler) => s.setReducedMotion(true)],
  ])("renders 0 pulse frames when %s", (_label, turnOff) => {
    const { render, scheduler } = createHarness();
    startPulse(scheduler);
    turnOff(scheduler);
    vi.advanceTimersByTime(2000);
    expect(render).toHaveBeenCalledTimes(0);
  });

  it("stops the pulse when a flag turns off mid-pulse", () => {
    const { render, scheduler } = createHarness();
    startPulse(scheduler);
    vi.advanceTimersByTime(1000);
    scheduler.setFocused(false);
    render.mockClear();
    vi.advanceTimersByTime(2000);
    expect(render).toHaveBeenCalledTimes(0);
  });

  it("holds an invalidate while hidden and renders it once shown", () => {
    const { render, scheduler } = createHarness();
    scheduler.setVisible(false);
    scheduler.invalidate();
    vi.advanceTimersByTime(1000);
    expect(render).toHaveBeenCalledTimes(0);
    scheduler.setVisible(true);
    vi.advanceTimersByTime(1000);
    expect(render).toHaveBeenCalledTimes(1);
  });

  it("renders nothing after dispose", () => {
    const { render, scheduler } = createHarness();
    startPulse(scheduler);
    scheduler.invalidate();
    scheduler.dispose();
    scheduler.invalidate();
    vi.advanceTimersByTime(2000);
    expect(render).toHaveBeenCalledTimes(0);
  });
});
