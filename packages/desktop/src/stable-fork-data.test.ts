import { mkdtempSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { findNonForkData, isForkDataLive } from "./stable-fork-data";

function createHome(label?: string): string {
  const home = mkdtempSync(path.join(os.tmpdir(), "paseo-stable-fork-"));
  if (label !== undefined) writeFileSync(path.join(home, ".paseo-switch-side"), label);
  return home;
}

describe("isForkDataLive", () => {
  it("accepts a home the switch script labelled as fork data", () => {
    expect(isForkDataLive(createHome("fork\n"))).toBe(true);
  });

  it("treats an unlabelled home as release data", () => {
    expect(isForkDataLive(createHome())).toBe(false);
  });

  it("rejects any other label", () => {
    expect(isForkDataLive(createHome("release\n"))).toBe(false);
  });

  it("rejects a home that does not exist", () => {
    expect(isForkDataLive(path.join(createHome(), "missing"))).toBe(false);
  });
});

describe("findNonForkData", () => {
  it("returns nothing when the home and userData both hold fork data", () => {
    expect(findNonForkData([createHome("fork\n"), createHome("fork\n")])).toEqual([]);
  });

  it("names the userData folder an interrupted switch left with release data", () => {
    const home = createHome("fork\n");
    const userData = createHome();

    expect(findNonForkData([home, userData])).toEqual([userData]);
  });

  it("names both folders when the app is opened on release data", () => {
    const home = createHome();
    const userData = createHome();

    expect(findNonForkData([home, userData])).toEqual([home, userData]);
  });
});
