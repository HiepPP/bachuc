import type { BranchInfo } from "../shared/branch";

export const MAX_LABEL = 24;

export function truncate(text: string, max = MAX_LABEL) {
  return text.length <= max ? text : `${text.slice(0, max - 1)}…`;
}

export function pillLabel(info: BranchInfo) {
  const name = info.detached ? `@${info.sha ?? "HEAD"}` : (info.branch ?? "unknown");
  return truncate(name);
}

export function pillTitle(info: BranchInfo) {
  const parts = [
    info.detached ? `Detached at ${info.sha ?? "HEAD"}` : `Branch ${info.branch ?? "unknown"}`,
  ];
  if (info.upstream) {
    parts.push(`tracks ${info.upstream}`);
    if (info.ahead !== null && info.behind !== null)
      parts.push(`ahead ${info.ahead} / behind ${info.behind}`);
  }
  if (info.dirty) parts.push("uncommitted changes");
  if (info.pr) parts.push(`PR #${info.pr.number} ${info.pr.state.toLowerCase()}`);
  return parts.join(" · ");
}
