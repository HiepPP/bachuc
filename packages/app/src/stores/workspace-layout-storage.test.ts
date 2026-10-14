import { describe, expect, it } from "vitest";
import { WorkspaceLayoutPersistedStateSchema } from "./workspace-layout-storage";

function layoutWithTabs(
  tabs: Array<{ tabId: string; target: unknown; createdAt: number }>,
  focusedTabId: string | null = tabs[0]?.tabId ?? null,
) {
  return {
    layoutByWorkspace: {
      "server-1:workspace-1": {
        root: {
          kind: "pane",
          pane: {
            id: "main",
            tabIds: tabs.map((tab) => tab.tabId),
            focusedTabId,
            tabs,
          },
        },
        focusedPaneId: "main",
      },
    },
  };
}

const agentTab = {
  tabId: "agent_agent-a",
  target: { kind: "agent", agentId: "agent-a" },
  createdAt: 1,
};
const filesTab = { tabId: "files", target: { kind: "files" }, createdAt: 2 };
const fleetMapTab = { tabId: "fleet_map", target: { kind: "fleet_map" }, createdAt: 3 };

describe("workspace layout storage schema", () => {
  it("parses a layout without retired tabs to an equal object", () => {
    const state = layoutWithTabs([agentTab, filesTab]);
    const result = WorkspaceLayoutPersistedStateSchema.safeParse(state);

    expect(result.success).toBe(true);
    expect(result.data).toEqual(state);
  });

  it("drops a saved fleet map tab and keeps the rest of the layout", () => {
    const result = WorkspaceLayoutPersistedStateSchema.safeParse(
      layoutWithTabs([agentTab, fleetMapTab, filesTab], "fleet_map"),
    );

    expect(result.success).toBe(true);
    expect(result.data).toEqual(layoutWithTabs([agentTab, filesTab], "agent_agent-a"));
  });

  it("leaves a pane with only a fleet map tab empty", () => {
    const result = WorkspaceLayoutPersistedStateSchema.safeParse(layoutWithTabs([fleetMapTab]));

    expect(result.success).toBe(true);
    expect(result.data).toEqual(layoutWithTabs([], null));
  });

  it("still rejects an unknown tab kind", () => {
    const state = layoutWithTabs([{ tabId: "x", target: { kind: "nope" }, createdAt: 1 }]);

    expect(WorkspaceLayoutPersistedStateSchema.safeParse(state).success).toBe(false);
  });
});
