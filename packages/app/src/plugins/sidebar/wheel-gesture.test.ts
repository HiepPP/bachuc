import { describe, expect, it } from "vitest";
import { createWheelGesture } from "./wheel-gesture";

describe("sidebar wheel gesture", () => {
  it("fires once per horizontal stroke, including its momentum tail", () => {
    const gesture = createWheelGesture();
    const fired = [10, 12, 14, 9, 6, 4, 3, 2, 1].map((dx, index) => gesture(dx, 0, index * 16));
    expect(fired.filter((value) => value !== 0)).toEqual([1]);
  });

  it("ignores mostly vertical scrolling and fires again after a pause", () => {
    const gesture = createWheelGesture();
    expect([10, 10, 10].map((dx, index) => gesture(dx, 40, index * 16))).toEqual([0, 0, 0]);
    expect([-15, -15].map((dx, index) => gesture(dx, 0, 1000 + index * 16))).toEqual([0, -1]);
  });
});
