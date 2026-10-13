import { describe, expect, it } from "vitest";
import { WorkspaceLayoutPersistedStateSchema } from "./workspace-layout-storage";

function layoutWithTabs(tabs: Array<{ tabId: string; target: unknown; createdAt: number }>) {
  return {
    layoutByWorkspace: {
      "server-1:workspace-1": {
        root: {
          kind: "pane",
          pane: {
            id: "main",
            tabIds: tabs.map((tab) => tab.tabId),
            focusedTabId: tabs[0]?.tabId ?? null,
            tabs,
          },
        },
        focusedPaneId: "main",
      },
    },
  };
}

// A layout in the shape builds wrote before the fleet_map kind existed.
const savedBeforeFleetMap = {
  pinnedAgentIdsByWorkspace: { "server-1:workspace-1": ["agent-a"] },
  ...layoutWithTabs([
    { tabId: "agent_agent-a", target: { kind: "agent", agentId: "agent-a" }, createdAt: 1 },
    { tabId: "files", target: { kind: "files" }, createdAt: 2 },
    { tabId: "pull_request", target: { kind: "pull_request" }, createdAt: 3 },
    { tabId: "file_src/a.ts", target: { kind: "file", path: "src/a.ts" }, createdAt: 4 },
  ]),
  explorerSidebarWidthByWorkspace: { "server-1:workspace-1": 320 },
  explorerPaneIdByWorkspace: { "server-1:workspace-1": null },
};

describe("workspace layout storage schema", () => {
  it("parses a layout saved before the fleet map kind to an equal object", () => {
    const result = WorkspaceLayoutPersistedStateSchema.safeParse(savedBeforeFleetMap);

    expect(result.success).toBe(true);
    expect(result.data).toEqual(savedBeforeFleetMap);
  });

  it("parses a fleet map tab", () => {
    const state = layoutWithTabs([
      { tabId: "fleet_map", target: { kind: "fleet_map" }, createdAt: 5 },
    ]);
    const result = WorkspaceLayoutPersistedStateSchema.safeParse(state);

    expect(result.success).toBe(true);
    expect(result.data).toEqual(state);
  });

  it("rejects a fleet map target with extra fields", () => {
    const state = layoutWithTabs([
      { tabId: "fleet_map", target: { kind: "fleet_map", serverId: "x" }, createdAt: 5 },
    ]);

    expect(WorkspaceLayoutPersistedStateSchema.safeParse(state).success).toBe(false);
  });
});
