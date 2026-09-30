import { describe, expect, it } from "vitest";
import {
  buildLegacyPluginSurfaceRedirectRoute,
  buildPluginSurfaceRoute,
  parsePluginSurfaceRoute,
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
