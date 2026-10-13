import { describe, expect, it } from "vitest";
import {
  deriveMacDockBadgeCount,
  deriveMacDockBadgeCountFromWorkspaceStatuses,
  isWorkspaceActionableForDesktopBadge,
  resolveDockBounceChange,
} from "./desktop-badge-state";

describe("desktop-badge-state", () => {
  it("treats attention-requiring workspace statuses as actionable", () => {
    expect(isWorkspaceActionableForDesktopBadge("attention")).toBe(true);
    expect(isWorkspaceActionableForDesktopBadge("needs_input")).toBe(true);
    expect(isWorkspaceActionableForDesktopBadge("failed")).toBe(true);
  });

  it("ignores running and done workspace statuses", () => {
    expect(isWorkspaceActionableForDesktopBadge("running")).toBe(false);
    expect(isWorkspaceActionableForDesktopBadge("done")).toBe(false);
  });

  it("returns undefined when no visible workspaces need attention", () => {
    expect(deriveMacDockBadgeCountFromWorkspaceStatuses(["done", "running"])).toBeUndefined();
  });

  it("counts only actionable visible workspaces", () => {
    expect(
      deriveMacDockBadgeCountFromWorkspaceStatuses([
        "done",
        "attention",
        "running",
        "needs_input",
        "failed",
      ]),
    ).toBe(3);
  });

  it("counts question threads instead of workspaces while a question waits", () => {
    expect(
      deriveMacDockBadgeCount({
        questionThreadCount: 2,
        workspaceStatuses: ["attention", "needs_input", "failed"],
      }),
    ).toBe(2);
  });

  it("falls back to actionable workspaces when no question waits", () => {
    expect(
      deriveMacDockBadgeCount({
        questionThreadCount: 0,
        workspaceStatuses: ["attention", "done"],
      }),
    ).toBe(1);
    expect(
      deriveMacDockBadgeCount({ questionThreadCount: 0, workspaceStatuses: ["done"] }),
    ).toBeUndefined();
  });

  it("bounces the dock for each new question and stops when none wait", () => {
    expect(resolveDockBounceChange(0, 1)).toBe("bounce");
    expect(resolveDockBounceChange(1, 2)).toBe("bounce");
    expect(resolveDockBounceChange(2, 1)).toBeNull();
    expect(resolveDockBounceChange(1, 1)).toBeNull();
    expect(resolveDockBounceChange(1, 0)).toBe("stop");
    expect(resolveDockBounceChange(0, 0)).toBeNull();
  });
});
