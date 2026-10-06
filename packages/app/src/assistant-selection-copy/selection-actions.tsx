export interface AssistantSelectionActionsProps {
  serverId: string;
  agentId: string;
  workspaceId?: string | null;
}

// Native text selection gives JS no selection events, so plugin selection actions are web-only.
export function AssistantSelectionActions(_props: AssistantSelectionActionsProps) {
  return null;
}
