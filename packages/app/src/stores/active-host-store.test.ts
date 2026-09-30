import { beforeEach, describe, expect, it } from "vitest";
import {
  getNextActiveServerId,
  resolveActiveServerId,
  useActiveHostStore,
} from "./active-host-store";

describe("active host store", () => {
  beforeEach(() => {
    useActiveHostStore.setState({
      activeServerId: null,
      previousServerId: null,
      lastWorkspaceIdByServerId: {},
      lastRouteByServerId: {},
    });
  });

  it("defaults to all machines", () => {
    expect(useActiveHostStore.getState().activeServerId).toBeNull();
  });

  it("resolves a known active host and treats an unknown one as all machines", () => {
    expect(resolveActiveServerId(null, ["a", "b"])).toBeNull();
    expect(resolveActiveServerId("a", ["a", "b"])).toBe("a");
    expect(resolveActiveServerId("gone", ["a", "b"])).toBeNull();
  });

  it("cycles to the next host, wrapping, and starts at the first from all hosts", () => {
    expect(getNextActiveServerId(["a", "b"], null)).toBe("a");
    expect(getNextActiveServerId(["a", "b"], "a")).toBe("b");
    expect(getNextActiveServerId(["a", "b"], "b")).toBe("a");
    expect(getNextActiveServerId(["a", "b"], "gone")).toBe("a");
    expect(getNextActiveServerId([], null)).toBeNull();
  });

  it("sets and clears the active host", () => {
    useActiveHostStore.getState().setActiveServerId("a");
    expect(useActiveHostStore.getState().activeServerId).toBe("a");
    useActiveHostStore.getState().setActiveServerId(null);
    expect(useActiveHostStore.getState().activeServerId).toBeNull();
  });

  it("remembers the previous host when switching hosts", () => {
    const { setActiveServerId } = useActiveHostStore.getState();
    setActiveServerId("a");
    expect(useActiveHostStore.getState().previousServerId).toBeNull();
    setActiveServerId("b");
    expect(useActiveHostStore.getState().previousServerId).toBe("a");
  });

  it("remembers the previous host when switching to all hosts", () => {
    const { setActiveServerId } = useActiveHostStore.getState();
    setActiveServerId("b");
    setActiveServerId(null);
    expect(useActiveHostStore.getState().activeServerId).toBeNull();
    expect(useActiveHostStore.getState().previousServerId).toBe("b");
  });

  it("keeps the previous host when the active host is set again", () => {
    const { setActiveServerId } = useActiveHostStore.getState();
    setActiveServerId("a");
    setActiveServerId("b");
    setActiveServerId("b");
    expect(useActiveHostStore.getState().previousServerId).toBe("a");
  });

  it("drops an unknown previous host on reconcile", () => {
    useActiveHostStore.setState({ activeServerId: "a", previousServerId: "gone" });
    useActiveHostStore.getState().reconcile(["a", "b"]);
    expect(useActiveHostStore.getState().activeServerId).toBe("a");
    expect(useActiveHostStore.getState().previousServerId).toBeNull();
  });

  it("keeps a known previous host on reconcile", () => {
    useActiveHostStore.setState({ activeServerId: "a", previousServerId: "b" });
    const before = useActiveHostStore.getState();
    before.reconcile(["a", "b"]);
    expect(useActiveHostStore.getState()).toBe(before);
  });

  it("remembers the last workspace per host", () => {
    const { rememberWorkspace } = useActiveHostStore.getState();
    rememberWorkspace("a", "w1");
    rememberWorkspace("b", "w2");
    rememberWorkspace("a", "w3");
    expect(useActiveHostStore.getState().lastWorkspaceIdByServerId).toEqual({ a: "w3", b: "w2" });
  });

  it("reconciles a removed host back to all machines and drops its memory", () => {
    useActiveHostStore.setState({
      activeServerId: "gone",
      lastWorkspaceIdByServerId: { gone: "w1", a: "w2" },
      lastRouteByServerId: { gone: "/h/gone/workspace/w1", a: "/h/a/workspace/w2" },
    });
    useActiveHostStore.getState().reconcile(["a"]);
    expect(useActiveHostStore.getState().activeServerId).toBeNull();
    expect(useActiveHostStore.getState().lastWorkspaceIdByServerId).toEqual({ a: "w2" });
    expect(useActiveHostStore.getState().lastRouteByServerId).toEqual({
      a: "/h/a/workspace/w2",
    });
  });

  it("remembers the last route per host", () => {
    const { rememberRoute } = useActiveHostStore.getState();
    rememberRoute("a", "/h/a/workspace/w1");
    rememberRoute("a", "/h/a/plugin/board/sidebar/board");
    expect(useActiveHostStore.getState().lastRouteByServerId).toEqual({
      a: "/h/a/plugin/board/sidebar/board",
    });
  });

  it("keeps state identity when reconcile changes nothing", () => {
    useActiveHostStore.setState({ activeServerId: "a", lastWorkspaceIdByServerId: { a: "w1" } });
    const before = useActiveHostStore.getState();
    before.reconcile(["a", "b"]);
    expect(useActiveHostStore.getState()).toBe(before);
  });
});
