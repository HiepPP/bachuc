import { describe, expect, it } from "vitest";
import { diffDemandServerIds, resolveSidebarDemandServerIds } from "./use-sidebar-workspaces-list";

const allServerIds = ["a", "b", "c"];

describe("resolveSidebarDemandServerIds", () => {
  it("keeps the previous host's demand on native while a host is active", () => {
    expect(
      resolveSidebarDemandServerIds({
        serverIds: ["b"],
        allServerIds,
        activeServerId: "b",
        previousServerId: "a",
        keepPrevious: true,
      }),
    ).toEqual(["b", "a"]);
  });

  it("returns the same reference when keepPrevious is false", () => {
    const serverIds = ["b"];
    expect(
      resolveSidebarDemandServerIds({
        serverIds,
        allServerIds,
        activeServerId: "b",
        previousServerId: "a",
        keepPrevious: false,
      }),
    ).toBe(serverIds);
  });

  it("returns the same reference when all hosts are shown", () => {
    const serverIds = ["a", "b", "c"];
    expect(
      resolveSidebarDemandServerIds({
        serverIds,
        allServerIds,
        activeServerId: null,
        previousServerId: "a",
        keepPrevious: true,
      }),
    ).toBe(serverIds);
  });

  it("ignores a previous host that is no longer known", () => {
    const serverIds = ["b"];
    expect(
      resolveSidebarDemandServerIds({
        serverIds,
        allServerIds,
        activeServerId: "b",
        previousServerId: "gone",
        keepPrevious: true,
      }),
    ).toBe(serverIds);
  });

  it("ignores a missing previous host", () => {
    const serverIds = ["b"];
    expect(
      resolveSidebarDemandServerIds({
        serverIds,
        allServerIds,
        activeServerId: "b",
        previousServerId: null,
        keepPrevious: true,
      }),
    ).toBe(serverIds);
  });

  it("does not duplicate a previous host that is already present", () => {
    const serverIds = ["b", "a"];
    expect(
      resolveSidebarDemandServerIds({
        serverIds,
        allServerIds,
        activeServerId: "b",
        previousServerId: "a",
        keepPrevious: true,
      }),
    ).toBe(serverIds);
  });
});

describe("diffDemandServerIds", () => {
  it("neither acquires nor releases hosts that stay", () => {
    expect(diffDemandServerIds(["a", "b"], ["a", "b"])).toEqual({ acquire: [], release: [] });
  });

  it("treats a reorder as a no-op", () => {
    expect(diffDemandServerIds(["b", "a"], ["a", "b"])).toEqual({ acquire: [], release: [] });
  });

  it("acquires only the new host when the previous host is added", () => {
    expect(diffDemandServerIds(["a"], ["b", "a"])).toEqual({ acquire: ["b"], release: [] });
  });

  it("acquires the new host and releases the old one when the set is replaced", () => {
    expect(diffDemandServerIds(["a"], ["b"])).toEqual({ acquire: ["b"], release: ["a"] });
  });

  it("releases everything when nothing is wanted", () => {
    expect(diffDemandServerIds(["a", "b"], [])).toEqual({ acquire: [], release: ["a", "b"] });
  });

  it("acquires a repeated host once", () => {
    expect(diffDemandServerIds([], ["a", "a"])).toEqual({ acquire: ["a"], release: [] });
  });
});
