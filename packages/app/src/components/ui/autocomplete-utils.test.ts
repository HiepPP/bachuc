import { describe, expect, it } from "vitest";

import {
  getAutocompleteFallbackIndex,
  getAutocompleteScrollOffset,
  getRelativeAnchorRect,
  orderAutocompleteOptions,
} from "./autocomplete-utils";

const OPTIONS = ["alpha", "beta", "gamma"];

describe("orderAutocompleteOptions", () => {
  it("keeps first logical option closest to the input by default", () => {
    expect(orderAutocompleteOptions(OPTIONS)).toEqual(["gamma", "beta", "alpha"]);
  });

  it("keeps normal top-down order when below-input is selected", () => {
    expect(orderAutocompleteOptions(OPTIONS, "below-input")).toEqual(["alpha", "beta", "gamma"]);
  });
});

describe("getAutocompleteFallbackIndex", () => {
  it("picks the option nearest the input by default", () => {
    expect(getAutocompleteFallbackIndex(3)).toBe(2);
    expect(getAutocompleteFallbackIndex(0)).toBe(-1);
  });

  it("picks top item when below-input ordering is used", () => {
    expect(getAutocompleteFallbackIndex(3, "below-input")).toBe(0);
  });
});

describe("getAutocompleteScrollOffset", () => {
  it("scrolls up when the active item is above the viewport", () => {
    expect(
      getAutocompleteScrollOffset({
        currentOffset: 120,
        viewportHeight: 80,
        itemTop: 90,
        itemHeight: 20,
      }),
    ).toBe(90);
  });

  it("scrolls down when the active item is below the viewport", () => {
    expect(
      getAutocompleteScrollOffset({
        currentOffset: 0,
        viewportHeight: 100,
        itemTop: 150,
        itemHeight: 24,
      }),
    ).toBe(74);
  });
});

describe("getRelativeAnchorRect", () => {
  const host = { x: 100, y: 0, width: 800, height: 600 };

  it("positions the anchor relative to the portal host", () => {
    expect(getRelativeAnchorRect({ x: 300, y: 450, width: 400, height: 80 }, host)).toEqual({
      x: 200,
      y: 450,
      width: 400,
      hostHeight: 600,
    });
  });

  it("ignores an anchor hidden with display none", () => {
    expect(getRelativeAnchorRect({ x: 0, y: 0, width: 0, height: 0 }, host)).toBeNull();
    expect(getRelativeAnchorRect({ x: 300, y: 450, width: 400, height: 0 }, host)).toBeNull();
  });

  it("ignores a hidden portal host", () => {
    const anchor = { x: 300, y: 450, width: 400, height: 80 };
    expect(getRelativeAnchorRect(anchor, { x: 0, y: 0, width: 0, height: 0 })).toBeNull();
  });
});
