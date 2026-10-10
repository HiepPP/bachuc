import { describe, expect, it } from "vitest";
import {
  buildLegacyPluginSurfaceRedirectRoute,
  buildPluginSurfaceRoute,
  isPluginOverlayRoute,
  isPluginOverlayRouteAbove,
  parsePluginSurfaceRoute,
  PLUGIN_SURFACE_SCREEN_NAME,
} from "./routes";

describe("buildPluginSurfaceRoute", () => {
  it("keeps direct surfaces and sidebar contributions in separate route namespaces", () => {
    expect(buildPluginSurfaceRoute("host/one", "review", { kind: "surface", id: "overview" })).toBe(
      "/h/host%2Fone/plugin/review/surface/overview",
    );
    expect(buildPluginSurfaceRoute("host/one", "review", { kind: "sidebar", id: "overview" })).toBe(
      "/h/host%2Fone/plugin/review/sidebar/overview",
    );
  });

  it("marks only an overlay presentation in the query", () => {
    const identity = { kind: "surface", id: "overview" } as const;
    expect(buildPluginSurfaceRoute("h", "review", identity, "overlay")).toBe(
      "/h/h/plugin/review/surface/overview?presentation=overlay",
    );
    expect(buildPluginSurfaceRoute("h", "review", identity, "page")).toBe(
      "/h/h/plugin/review/surface/overview",
    );
    expect(
      parsePluginSurfaceRoute("/h/h/plugin/review/surface/overview?presentation=overlay"),
    ).toEqual({ serverId: "h", pluginId: "review", identity });
  });

  it("redirects legacy plugin surface URLs to their sidebar contribution identity", () => {
    expect(buildLegacyPluginSurfaceRedirectRoute("host/one", "review", "overview/item")).toBe(
      "/h/host%2Fone/plugin/review/sidebar/overview%2Fitem",
    );
  });
});

describe("parsePluginSurfaceRoute", () => {
  it("round-trips a built route", () => {
    const route = buildPluginSurfaceRoute("host/one", "review", { kind: "sidebar", id: "a/b" });
    expect(parsePluginSurfaceRoute(route)).toEqual({
      serverId: "host/one",
      pluginId: "review",
      identity: { kind: "sidebar", id: "a/b" },
    });
  });

  it("ignores routes that are not plugin surfaces", () => {
    expect(parsePluginSurfaceRoute("/h/a/workspace/w1")).toBeNull();
    expect(parsePluginSurfaceRoute("/sessions")).toBeNull();
  });
});

describe("plugin overlay routes in a stack", () => {
  const workspace = { key: "w", name: "workspace/[workspaceId]/index" };
  const sessions = { key: "s", name: "sessions" };
  const overlay = {
    key: "o",
    name: PLUGIN_SURFACE_SCREEN_NAME,
    params: { presentation: "overlay" },
  };
  const page = { key: "p", name: PLUGIN_SURFACE_SCREEN_NAME, params: {} };

  it("is an overlay only with a screen below it", () => {
    expect(isPluginOverlayRoute({ routes: [workspace, overlay] }, "o")).toBe(true);
    expect(isPluginOverlayRoute({ routes: [overlay] }, "o")).toBe(false);
    expect(isPluginOverlayRoute({ routes: [workspace, page] }, "p")).toBe(false);
    expect(
      isPluginOverlayRoute({ routes: [workspace, { ...sessions, params: overlay.params }] }, "s"),
    ).toBe(false);
    expect(isPluginOverlayRoute({ routes: [workspace, overlay] }, "missing")).toBe(false);
  });

  it("finds the overlay only on the route directly below it", () => {
    const state = { routes: [workspace, sessions, overlay] };
    expect(isPluginOverlayRouteAbove(state, "s")).toBe(true);
    expect(isPluginOverlayRouteAbove(state, "w")).toBe(false);
    expect(isPluginOverlayRouteAbove(state, "o")).toBe(false);
    expect(isPluginOverlayRouteAbove({ routes: [overlay] }, "o")).toBe(false);
  });
});
