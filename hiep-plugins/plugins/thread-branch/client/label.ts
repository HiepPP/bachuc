import type { BranchInfo } from "../shared/branch";

export const MAX_LABEL = 24;

export function truncate(text: string, max = MAX_LABEL) {
  return text.length <= max ? text : `${text.slice(0, max - 1)}…`;
}

export function pillLabel(info: BranchInfo) {
  const name = info.detached ? `@${info.sha ?? "HEAD"}` : (info.branch ?? "unknown");
  return truncate(name);
}

/** New workspace label: the branch of the selected project, before any agent exists. */
export function draftPillLabel(cwd: string | undefined, info: BranchInfo | undefined) {
  if (!cwd) return "No project";
  if (!info) return "…";
  return info.repo ? pillLabel(info) : "Not a git repo";
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
