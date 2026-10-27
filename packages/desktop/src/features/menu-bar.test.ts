import { describe, expect, it } from "vitest";
import { buildMenuBarItems, needsDaemonStart } from "./menu-bar.js";

describe("menu bar", () => {
  it("shows a running daemon with its listen address and allows a restart", () => {
    const items = buildMenuBarItems({ status: "running", listen: "127.0.0.1:6767" });
    expect(items[0]).toEqual({
      id: "status",
      label: "Daemon running · 127.0.0.1:6767",
      enabled: false,
    });
    expect(items.find((item) => item.id === "restart")?.enabled).toBe(true);
    expect(items.map((item) => item.id)).toEqual([
      "status",
      "open-gpui",
      "restart",
      "logs",
      "quit",
    ]);
  });

  it("shows a stopped daemon and offers no restart", () => {
    for (const status of ["stopped", "errored"] as const) {
      const items = buildMenuBarItems({ status, listen: null });
      expect(items[0].label).toBe("Daemon stopped");
      expect(items.find((item) => item.id === "restart")?.enabled).toBe(false);
    }
  });

  it("starts the daemon only when it is not running or starting", () => {
    expect(needsDaemonStart({ status: "running", listen: "127.0.0.1:6767" })).toBe(false);
    expect(needsDaemonStart({ status: "starting", listen: null })).toBe(false);
    expect(needsDaemonStart({ status: "stopped", listen: null })).toBe(true);
    expect(needsDaemonStart({ status: "errored", listen: null })).toBe(true);
  });
});
