import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import type { PluginButton } from "@getpaseo/plugin/client";
import type { InstalledPlugin } from "../types";
import {
  PluginButtonStore,
  buttonInToolbar,
  buttonMatches,
  resolveButtonForContext,
} from "./model";

function installation(): InstalledPlugin {
  return {
    lifetime: new AbortController(),
    id: "review",
    serverId: "host-a",
    clientBundle: "bundle",
    queryClient: new QueryClient(),
    cleanup: () => undefined,
    settingsScreens: [],
    surfaces: [],
    sidebarItems: [],
    workspacePanels: [],
    commandCenterItems: [],
    clientSlashCommands: [],
    attachmentSources: [],
    themes: [],
    timelineTransformers: [],
    timelineRenderers: [],
  };
}

function store() {
  return new PluginButtonStore({
    validateIconName(name) {
      if (name !== "Scan") throw new Error("Unknown icon");
    },
  });
}

function button(onPress = () => {}): PluginButton {
  return { title: "Review", icon: "Scan", behavior: { kind: "action", onPress } };
}

describe("plugin buttons", () => {
  it("updates visibility in place, closes hidden content, and scopes each placement", () => {
    const buttons = store();
    const plugin = installation();
    const header = buttons.addHeaderButton(plugin, {
      id: "review",
      workspaceId: "workspace",
      button: button(),
    });
    buttons.addComposerPill(plugin, {
      id: "review",
      workspaceId: "workspace",
      agentId: "agent",
      button: button(),
    });
    const first = buttons.getSnapshot()[0];
    expect(buttonMatches(first, "host-a", "workspace", null)).toBe(true);
    expect(buttonMatches(first, "host-b", "workspace", null)).toBe(false);
    expect(buttonMatches(first, "host-a", "another-workspace", null)).toBe(false);
    expect(buttonMatches(first, "host-a", "workspace", "agent")).toBe(false);
    expect(buttonMatches(buttons.getSnapshot()[1], "host-a", "workspace", "agent")).toBe(true);
    expect(buttonMatches(buttons.getSnapshot()[1], "host-a", "workspace", "other-agent")).toBe(
      false,
    );
    buttons.setOpen(first.key, true);
    header.update({ visible: false, label: "3 reviews" });
    expect(buttonMatches(buttons.getSnapshot()[0], "host-a", "workspace", null)).toBe(false);
    expect(buttons.getSnapshot()[0].open).toBe(false);
    header.update({ visible: true });
    expect(buttons.getSnapshot().map((entry) => entry.key)).toEqual([first.key, first.key + 1]);
    expect(buttons.getSnapshot()[0].button.label).toBe("3 reviews");
    header.remove();
    header.remove();
    header.update({ visible: true });
    expect(buttons.getSnapshot().map((entry) => entry.placement)).toEqual(["composer"]);
  });

  it("rejects duplicates within a target and validates updates atomically", () => {
    const buttons = store();
    const plugin = installation();
    const contribution = { id: "review", workspaceId: "workspace", button: button() };
    const header = buttons.addHeaderButton(plugin, contribution);
    expect(() => buttons.addHeaderButton(plugin, contribution)).toThrow(
      "Duplicate plugin button: review",
    );
    expect(() => buttons.addHeaderButton(installation(), contribution)).not.toThrow();
    expect(() =>
      buttons.addHeaderButton(plugin, { ...contribution, workspaceId: "other" }),
    ).not.toThrow();
    expect(() => header.update({ title: "Changed", icon: "Invalid" })).toThrow("Unknown icon");
    expect(buttons.getSnapshot()[0].button.title).toBe("Review");
  });

  it("prevents repeated presses and cannot resurrect a removed pending button", async () => {
    const buttons = store();
    let finish = () => {};
    const pending = new Promise<void>((resolve) => {
      finish = resolve;
    });
    let calls = 0;
    const registration = buttons.addHeaderButton(installation(), {
      id: "review",
      workspaceId: "workspace",
      button: button(() => {
        calls++;
        return pending;
      }),
    });
    const key = buttons.getSnapshot()[0].key;
    const action = buttons.run(key);
    await buttons.run(key);
    expect(calls).toBe(1);
    expect(buttons.getSnapshot()[0].pending).toBe(true);
    registration.remove();
    finish();
    await action;
    expect(buttons.getSnapshot()).toEqual([]);
  });

  it("allows retry after failure and respects disabled actions and menu ancestors", async () => {
    const buttons = store();
    let calls = 0;
    const registration = buttons.addHeaderButton(installation(), {
      id: "review",
      workspaceId: "workspace",
      button: button(() => {
        if (++calls === 1) throw new Error("Review unavailable");
      }),
    });
    const key = buttons.getSnapshot()[0].key;
    await expect(buttons.run(key)).rejects.toThrow("Review unavailable");
    expect(buttons.getSnapshot()[0].pending).toBe(false);
    await buttons.run(key);
    expect(calls).toBe(2);
    registration.update({ disabled: true });
    await buttons.run(key);
    expect(calls).toBe(2);
    registration.update({
      disabled: false,
      behavior: {
        kind: "menu",
        items: [
          {
            kind: "item",
            id: "nested",
            title: "Nested",
            disabled: true,
            behavior: {
              kind: "menu",
              items: [
                {
                  kind: "item",
                  id: "run",
                  title: "Run",
                  behavior: {
                    kind: "action",
                    onPress() {
                      calls++;
                    },
                  },
                },
              ],
            },
          },
        ],
      },
    });
    await buttons.run(key, ["nested", "run"]);
    expect(calls).toBe(2);
  });

  it("shows a shared composer pill on every agent and passes each agent to its action", async () => {
    const buttons = store();
    const seen: unknown[] = [];
    buttons.addComposerPill(installation(), {
      id: "skills",
      button: {
        title: "Skills",
        icon: "Scan",
        behavior: {
          kind: "action",
          onPress: (context) => {
            seen.push(context);
          },
        },
      },
    });
    const shared = buttons.getSnapshot()[0];
    expect(buttonMatches(shared, "host-a", "workspace-1", "agent-1")).toBe(true);
    expect(buttonMatches(shared, "host-a", "workspace-2", "agent-2")).toBe(true);
    expect(buttonMatches(shared, "host-a", "workspace-1", null)).toBe(false);
    expect(buttonMatches(shared, "host-b", "workspace-1", "agent-1")).toBe(false);
    const first = resolveButtonForContext(shared, "workspace-1", "agent-1");
    const second = resolveButtonForContext(shared, "workspace-2", "agent-2");
    await buttons.run(shared.key, [], first.context);
    await buttons.run(shared.key, [], second.context);
    expect(seen).toEqual([
      { context: "agent", workspaceId: "workspace-1", agentId: "agent-1" },
      { context: "agent", workspaceId: "workspace-2", agentId: "agent-2" },
    ]);
  });

  it("opens a shared composer pill only for the agent that opened it", () => {
    const buttons = store();
    buttons.addComposerPill(installation(), {
      id: "skills",
      button: {
        title: "Skills",
        icon: "Scan",
        behavior: { kind: "menu", items: [] },
      },
    });
    buttons.setOpen(buttons.getSnapshot()[0].key, true, "agent-1");
    const shared = buttons.getSnapshot()[0];
    expect(resolveButtonForContext(shared, "workspace", "agent-1").open).toBe(true);
    expect(resolveButtonForContext(shared, "workspace", "agent-2").open).toBe(false);
  });

  it("keeps a toolbar pill in the toolbar on wide layouts and in the track bar on compact", () => {
    const buttons = store();
    const plugin = installation();
    buttons.addComposerPill(plugin, { id: "skills", placement: "toolbar", button: button() });
    buttons.addComposerPill(plugin, { id: "review", button: button() });
    const [toolbar, track] = buttons.getSnapshot();
    expect(buttonInToolbar(toolbar, false)).toBe(true);
    expect(buttonInToolbar(toolbar, true)).toBe(false);
    expect(buttonInToolbar(track, false)).toBe(false);
  });

  it("lets a composer pill omit its icon and keeps it required for a header button", () => {
    const buttons = store();
    const plugin = installation();
    const { icon: _icon, ...iconless } = button();
    const pill = buttons.addComposerPill(plugin, { id: "mode", button: iconless });
    expect(buttons.getSnapshot()[0].button.icon).toBeUndefined();
    pill.update({ label: "Caveman: Ultra" });
    expect(buttons.getSnapshot()[0].button.label).toBe("Caveman: Ultra");
    expect(() =>
      buttons.addHeaderButton(plugin, {
        id: "review",
        workspaceId: "workspace",
        button: iconless as PluginButton,
      }),
    ).toThrow("Plugin button needs icon");
  });

  it("accepts a component label and rejects an empty or invalid one", () => {
    const buttons = store();
    const plugin = installation();
    const Label = () => "Caveman: Ultra";
    buttons.addComposerPill(plugin, { id: "mode", button: { ...button(), label: Label } });
    expect(buttons.getSnapshot()[0].button.label).toBe(Label);
    expect(() =>
      buttons.addComposerPill(plugin, { id: "empty", button: { ...button(), label: " " } }),
    ).toThrow("Plugin button needs label");
    expect(() =>
      buttons.addComposerPill(plugin, {
        id: "invalid",
        button: { ...button(), label: 3 as unknown as string },
      }),
    ).toThrow("Plugin button label must be text or a component");
  });
});
