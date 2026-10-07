import { describe, expect, it } from "vitest";
import {
  AUTO_MAX_HEIGHT,
  AUTO_MIN_HEIGHT,
  INITIAL_AUTO_HEIGHT,
  nextAutoHeight,
  readReportedHeight,
  type AutoHeight,
} from "./html-frame-height";

function report(...heights: number[]): AutoHeight {
  return heights.reduce(nextAutoHeight, INITIAL_AUTO_HEIGHT);
}

describe("readReportedHeight", () => {
  it("reads a web message object or a native JSON string", () => {
    expect(readReportedHeight({ paseoHtmlFrameHeight: 640 })).toBe(640);
    expect(readReportedHeight('{"paseoHtmlFrameHeight":512}')).toBe(512);
  });

  it("ignores anything else a page posts", () => {
    expect(readReportedHeight("not json")).toBeNull();
    expect(readReportedHeight({ paseoHtmlFrameHeight: "900" })).toBeNull();
    expect(readReportedHeight({ paseoHtmlFrameHeight: Number.NaN })).toBeNull();
    expect(readReportedHeight(null)).toBeNull();
  });
});

describe("nextAutoHeight", () => {
  it("grows and shrinks the frame to the page's content", () => {
    expect(report(1200).height).toBe(1200);
    expect(report(1200, 300).height).toBe(300);
  });

  it("clamps an untrusted height", () => {
    expect(report(1e9).height).toBe(AUTO_MAX_HEIGHT);
    expect(report(-50).height).toBe(AUTO_MIN_HEIGHT);
  });

  it("stops a page whose height follows the frame after three equal steps", () => {
    const state = report(136, 152, 168, 184, 200);

    expect(state.frozen).toBe(true);
    expect(state.height).toBe(152);
  });

  it("keeps following a page that grows by different steps, such as a chart loading", () => {
    const state = report(400, 640, 650, 900);

    expect(state).toMatchObject({ height: 900, frozen: false });
  });
});
