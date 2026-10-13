/**
 * @vitest-environment jsdom
 */
import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PARENT_AGENT_ID_LABEL } from "@getpaseo/protocol/agent-labels";
import { useSessionStore, type Agent, type WorkspaceDescriptor } from "@/stores/session-store";
import { selectFleetNodes } from "./nodes";
import { useFleetNodes } from "./use-fleet-nodes";

function makeAgent(input: {
  id: string;
  workspaceId?: string;
  cwd?: string;
  title?: string | null;
  status?: Agent["status"];
  createdAt?: string;
  pendingPermissionCount?: number;
  archivedAt?: string | null;
  parentId?: string;
  lastActivityAt?: string;
}): Agent {
  const status = input.status ?? "idle";
  return {
    serverId: "host-a",
    id: input.id,
    provider: "codex",
    status,
    turn:
      status === "running"
        ? { phase: "open", turnId: "turn-1", startedAt: null, cancellationRequestId: null }
        : { phase: "idle", cancellationRequestId: null },
    createdAt: new Date(input.createdAt ?? "2026-01-01T00:00:00.000Z"),
    updatedAt: new Date("2026-01-01T00:00:00.000Z"),
    lastUserMessageAt: null,
    lastActivityAt: new Date(input.lastActivityAt ?? "2026-01-01T00:00:00.000Z"),
    capabilities: {
      supportsStreaming: true,
      supportsSessionPersistence: true,
      supportsDynamicModes: true,
      supportsMcpServers: true,
      supportsReasoningStream: true,
      supportsToolInvocations: true,
    },
    currentModeId: null,
    availableModes: [],
    pendingPermissions: Array.from({ length: input.pendingPermissionCount ?? 0 }, (_, index) => ({
      id: `permission-${index}`,
      provider: "codex",
      name: "shell",
      kind: "tool",
      input: {},
    })),
    persistence: null,
    title: input.title === undefined ? `Title ${input.id}` : input.title,
    cwd: input.cwd ?? "/repo",
    workspaceId: input.workspaceId,
    model: null,
    archivedAt: input.archivedAt ? new Date(input.archivedAt) : null,
    parentAgentId: input.parentId ?? null,
    labels: input.parentId ? { [PARENT_AGENT_ID_LABEL]: input.parentId } : {},
  };
}

function makeWorkspace(id: string, name: string): WorkspaceDescriptor {
  return { id, name } as WorkspaceDescriptor;
}

function session(agents: Agent[], workspaces: WorkspaceDescriptor[] = []) {
  return {
    agents: new Map(agents.map((agent) => [agent.id, agent])),
    workspaces: new Map(workspaces.map((workspace) => [workspace.id, workspace])),
  };
}

describe("selectFleetNodes", () => {
  it("returns an empty list for a missing session", () => {
    expect(selectFleetNodes(undefined)).toEqual([]);
  });

  it("maps each non-archived agent to one node", () => {
    const nodes = selectFleetNodes(
      session(
        [
          makeAgent({ id: "a", workspaceId: "w1", createdAt: "2026-01-02T00:00:00.000Z" }),
          makeAgent({ id: "b", workspaceId: "w1", archivedAt: "2026-01-03T00:00:00.000Z" }),
        ],
        [makeWorkspace("w1", "main")],
      ),
    );

    expect(nodes).toEqual([
      {
        id: "a",
        title: "Title a",
        workspaceId: "w1",
        workspaceName: "main",
        parentId: null,
        bucket: "done",
        createdAt: new Date("2026-01-02T00:00:00.000Z"),
      },
    ]);
  });

  it("sets parentId for subagents and null for roots", () => {
    const nodes = selectFleetNodes(
      session([
        makeAgent({ id: "root", workspaceId: "w1" }),
        makeAgent({ id: "child", workspaceId: "w1", parentId: "root" }),
      ]),
    );

    expect(nodes.find((node) => node.id === "root")?.parentId).toBeNull();
    expect(nodes.find((node) => node.id === "child")?.parentId).toBe("root");
  });

  it("derives the sidebar bucket", () => {
    const nodes = selectFleetNodes(
      session([
        makeAgent({ id: "input", workspaceId: "w1", pendingPermissionCount: 1 }),
        makeAgent({ id: "run", workspaceId: "w1", status: "running" }),
      ]),
    );

    expect(nodes.find((node) => node.id === "input")?.bucket).toBe("needs_input");
    expect(nodes.find((node) => node.id === "run")?.bucket).toBe("running");
  });

  it("falls back to cwd when the workspace id is missing and leaves a missing name null", () => {
    const [node] = selectFleetNodes(session([makeAgent({ id: "a", cwd: "/repo/x" })]));

    expect(node?.workspaceId).toBe("/repo/x");
    expect(node?.workspaceName).toBeNull();
  });
});

describe("useFleetNodes", () => {
  afterEach(() => {
    useSessionStore.setState({ sessions: {} });
  });

  function setSession(serverId: string, agents: Agent[]) {
    useSessionStore.setState((state) => ({
      sessions: { ...state.sessions, [serverId]: session(agents) as never },
    }));
  }

  it("reads only the requested host", () => {
    setSession("host-a", [makeAgent({ id: "a", workspaceId: "w1" })]);
    setSession("host-b", [makeAgent({ id: "b", workspaceId: "w2" })]);

    const { result } = renderHook(() => useFleetNodes("host-a"));

    expect(result.current.map((node) => node.id)).toEqual(["a"]);
  });

  it("keeps the output reference across 100 timeline-only updates", () => {
    setSession("host-a", [makeAgent({ id: "a", workspaceId: "w1", status: "running" })]);
    let renders = 0;
    const { result } = renderHook(() => {
      renders += 1;
      return useFleetNodes("host-a");
    });
    const initial = result.current;
    const initialRenders = renders;
    let referenceChanges = 0;

    for (let i = 0; i < 100; i += 1) {
      act(() => {
        setSession("host-a", [
          makeAgent({
            id: "a",
            workspaceId: "w1",
            status: "running",
            lastActivityAt: new Date(Date.UTC(2026, 1, 1, 0, 0, i + 1)).toISOString(),
          }),
        ]);
      });
      if (result.current !== initial) referenceChanges += 1;
    }

    expect(referenceChanges).toBe(0);
    expect(renders).toBe(initialRenders);
  });

  it("returns a new output when the status changes", () => {
    setSession("host-a", [makeAgent({ id: "a", workspaceId: "w1", status: "running" })]);
    const { result } = renderHook(() => useFleetNodes("host-a"));
    const initial = result.current;

    act(() => {
      setSession("host-a", [makeAgent({ id: "a", workspaceId: "w1", pendingPermissionCount: 1 })]);
    });

    expect(result.current).not.toBe(initial);
    expect(result.current[0]?.bucket).toBe("needs_input");
  });
});
