import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import { buildEffectiveBindings, type KeyboardShortcutInput } from "@/keyboard/keyboard-shortcuts";
import { isPluginSurfacePath, resolvePluginShortcut } from "./shortcuts";
import type { InstalledPlugin } from "./types";

function installation(serverId: string, combo: string): InstalledPlugin {
  return {
    id: "watchtower-board",
    serverId,
    clientBundle: serverId,
    lifetime: new AbortController(),
    queryClient: new QueryClient(),
    cleanup: () => undefined,
    settingsScreens: [],
    surfaces: [{ id: "watchtower", Component: () => null }],
    sidebarItems: [],
    workspacePanels: [],
    commandCenterItems: [],
    clientSlashCommands: [],
    attachmentSources: [],
    themes: [],
    timelineTransformers: [],
    timelineRenderers: [],
    shortcuts: [{ id: "toggle", combo, surface: "watchtower", onPress: () => undefined }],
  };
}

function press(key: string, modifiers: Partial<KeyboardShortcutInput> = {}): KeyboardShortcutInput {
  return {
    key,
    code: `Key${key.toUpperCase()}`,
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    repeat: false,
    ...modifiers,
  };
}

const hostBindings = buildEffectiveBindings({});
const workspacePath = "/h/srv_1/workspace/wks_1";
const boardPath = "/h/srv_1/plugin/watchtower-board/surface/watchtower";

describe("plugin shortcuts", () => {
  it("presses on Cmd+E, and closes the surface when it is the current screen", () => {
    const plugins = [installation("srv_1", "Mod+E")];
    const input = { plugins, activeServerId: "srv_1", hostBindings, isMac: true };
    const event = press("e", { metaKey: true });
    expect(resolvePluginShortcut({ ...input, event, pathname: workspacePath })?.kind).toBe("press");
    expect(resolvePluginShortcut({ ...input, event, pathname: boardPath })?.kind).toBe("close");
    // Off macOS, Mod is Ctrl.
    expect(
      resolvePluginShortcut({
        ...input,
        isMac: false,
        event: press("e", { ctrlKey: true }),
        pathname: workspacePath,
      })?.kind,
    ).toBe("press");
  });

  it("ignores other hosts and combos that a host shortcut uses", () => {
    const event = press("e", { metaKey: true });
    expect(
      resolvePluginShortcut({
        plugins: [installation("srv_2", "Mod+E")],
        activeServerId: "srv_1",
        hostBindings,
        event,
        isMac: true,
        pathname: workspacePath,
      }),
    ).toBeNull();
    // Cmd+Shift+F toggles the Explorer sidebar, so a plugin cannot take it.
    expect(
      resolvePluginShortcut({
        plugins: [installation("srv_1", "Mod+Shift+F")],
        activeServerId: "srv_1",
        hostBindings,
        event: press("f", { metaKey: true, shiftKey: true }),
        isMac: true,
        pathname: workspacePath,
      }),
    ).toBeNull();
  });

  it("matches a surface path by plugin and surface id", () => {
    expect(isPluginSurfacePath(boardPath, "watchtower-board", "watchtower")).toBe(true);
    expect(isPluginSurfacePath(boardPath, "board", "watchtower")).toBe(false);
  });
});
