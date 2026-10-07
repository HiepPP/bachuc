import { describe, expect, it } from "vitest";
import {
  AUTO_INITIAL_HEIGHT,
  AUTO_MAX_HEIGHT,
  AUTO_MIN_HEIGHT,
  frameHeightKey,
  INITIAL_AUTO_HEIGHT,
  initialAutoHeight,
  nextAutoHeight,
  readReportedHeight,
  rememberAutoHeight,
  type AutoHeight,
} from "./html-frame-height";

describe("remembered frame heights", () => {
  it("keys a page by its content, not by identity", () => {
    expect(frameHeightKey("<p>a</p>")).toBe(frameHeightKey("<p>a</p>"));
    expect(frameHeightKey("<p>a</p>")).not.toBe(frameHeightKey("<p>b</p>"));
  });

  it("starts a page shown before at its last height, and a new page at the default", () => {
    rememberAutoHeight("seen-page", { ...INITIAL_AUTO_HEIGHT, height: 1478 });

    expect(initialAutoHeight("seen-page").height).toBe(1478);
    expect(initialAutoHeight("new-page").height).toBe(AUTO_INITIAL_HEIGHT);
  });

  it("starts a page that froze as frozen, so a remount does not creep taller", () => {
    rememberAutoHeight("vh-page", { ...INITIAL_AUTO_HEIGHT, height: 152, frozen: true });

    expect(initialAutoHeight("vh-page")).toMatchObject({ height: 152, frozen: true });
  });

  it("forgets the oldest page past 200 remembered pages", () => {
    rememberAutoHeight("first-page", { ...INITIAL_AUTO_HEIGHT, height: 500 });
    for (let index = 0; index < 200; index += 1) {
      rememberAutoHeight(`filler-${index}`, { ...INITIAL_AUTO_HEIGHT, height: 300 });
    }

    expect(initialAutoHeight("first-page").height).toBe(AUTO_INITIAL_HEIGHT);
    expect(initialAutoHeight("filler-199").height).toBe(300);
  });
});

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
