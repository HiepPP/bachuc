import type { WorkspaceDescriptor } from "@/stores/session-store";

export type DesktopBadgeWorkspaceStatus = WorkspaceDescriptor["status"];

export function isWorkspaceActionableForDesktopBadge(status: DesktopBadgeWorkspaceStatus): boolean {
  return status === "attention" || status === "needs_input" || status === "failed";
}

export function deriveMacDockBadgeCountFromWorkspaceStatuses(
  statuses: readonly DesktopBadgeWorkspaceStatus[],
): number | undefined {
  const actionableCount = statuses.filter(isWorkspaceActionableForDesktopBadge).length;
  return actionableCount > 0 ? actionableCount : undefined;
}

// A waiting question blocks its thread until the user answers, so while any
// question waits the badge counts those threads alone.
export function deriveMacDockBadgeCount(input: {
  questionThreadCount: number;
  workspaceStatuses: readonly DesktopBadgeWorkspaceStatus[];
}): number | undefined {
  if (input.questionThreadCount > 0) {
    return input.questionThreadCount;
  }
  return deriveMacDockBadgeCountFromWorkspaceStatuses(input.workspaceStatuses);
}

export function resolveDockBounceChange(
  previousQuestionThreadCount: number,
  questionThreadCount: number,
): "bounce" | "stop" | null {
  if (questionThreadCount > previousQuestionThreadCount) {
    return "bounce";
  }
  if (questionThreadCount === 0 && previousQuestionThreadCount > 0) {
    return "stop";
  }
  return null;
}
