import { getParentAgentIdFromLabels } from "@getpaseo/protocol/agent-labels";
import type { SessionState } from "@/stores/session-store";
import { deriveSidebarStateBucket, type SidebarStateBucket } from "@/utils/sidebar-agent-state";
import type { FleetLayoutNode } from "./layout";

export interface FleetNode extends FleetLayoutNode {
  title: string | null;
  workspaceName: string | null;
  bucket: SidebarStateBucket;
  createdAt: Date;
}

type FleetSession = Pick<SessionState, "agents" | "workspaces">;

const EMPTY_NODES: FleetNode[] = [];

export function selectFleetNodes(session: FleetSession | undefined): FleetNode[] {
  if (!session || session.agents.size === 0) {
    return EMPTY_NODES;
  }

  const nodes: FleetNode[] = [];
  for (const agent of session.agents.values()) {
    if (agent.archivedAt) {
      continue;
    }
    // Agents with no workspace id are grouped by their directory.
    const workspaceId = agent.workspaceId ?? agent.cwd;
    nodes.push({
      id: agent.id,
      title: agent.title,
      workspaceId,
      workspaceName: session.workspaces.get(workspaceId)?.name ?? null,
      parentId: getParentAgentIdFromLabels(agent.labels),
      bucket: deriveSidebarStateBucket({
        status: agent.turn.phase === "open" ? "running" : agent.status,
        pendingPermissionCount: agent.pendingPermissions.length,
        requiresAttention: agent.requiresAttention,
        attentionReason: agent.attentionReason,
      }),
      createdAt: agent.createdAt,
    });
  }
  return nodes;
}
