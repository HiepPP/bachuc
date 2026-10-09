export const ACTIVE_POLL_MS = 2500;
export const JEV_REVIEWING = "Jev reviewing...";

/** Only a Jev review changes server state without agent activity; everything else refetches on it. */
export function inspectionInterval(snapshot: { note: string } | undefined): number | false {
  return snapshot?.note === JEV_REVIEWING ? ACTIVE_POLL_MS : false;
}

/** A closed runtime is at rest: Send resumes it. */
export function agentBusy(
  agent: { status: string; attentionReason: string | null } | null,
): boolean {
  if (!agent) return false;
  return !["idle", "closed"].includes(agent.status) || agent.attentionReason === "permission";
}

export function inspectionActivity(agent: { status: string; lastActivityAt: string }) {
  return {
    status: agent.status,
    lastActivityAt: agent.status === "running" ? null : agent.lastActivityAt,
  };
}
