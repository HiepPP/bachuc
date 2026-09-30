import type { PluginButton } from "@getpaseo/plugin/client";
import type { BranchInfo } from "../shared/branch";
import { pillLabel, pillTitle } from "./label";
import { refTitle, refUrl, type GitRef } from "./refs";
import { syncIcons, type SyncState } from "./sync-icon";

export function describeBranchPill(
  info: BranchInfo,
  actions: { copy(): Promise<void>; fetch(): Promise<void>; refresh(): Promise<void> },
): PluginButton {
  return {
    title: pillTitle(info),
    icon: "GitBranch",
    label: pillLabel(info),
    visible: info.repo,
    behavior: {
      kind: "menu",
      items: [
        {
          kind: "item",
          id: "copy-branch",
          title: "Copy branch name",
          icon: "Copy",
          visible: !info.detached && info.branch !== null,
          behavior: { kind: "action", onPress: actions.copy },
        },
        { kind: "separator", id: "refresh-divider" },
        {
          kind: "item",
          id: "fetch",
          title: "Fetch",
          icon: "CloudDownload",
          visible: info.upstream !== null,
          behavior: { kind: "action", onPress: actions.fetch },
        },
        {
          kind: "item",
          id: "refresh",
          title: "Refresh",
          icon: "RefreshCw",
          behavior: { kind: "action", onPress: actions.refresh },
        },
      ],
    },
  };
}

/** One click opens the pull request; hidden when the branch has none. */
export function describePrPill(info: BranchInfo, openPr: () => Promise<void>): PluginButton {
  const pr = info.pr;
  return {
    title: pr ? `Open PR #${pr.number} (${pr.state.toLowerCase()})` : "Open PR",
    icon: "GitPullRequest",
    label: pr ? `#${pr.number}` : "PR",
    visible: info.repo && pr !== null,
    behavior: { kind: "action", onPress: openPr },
  };
}

/** One click opens the repository page of `origin`; hidden when there is no web remote. */
export function describeRepoPill(info: BranchInfo, openRepo: () => Promise<void>): PluginButton {
  const url = info.remoteUrl;
  const label = url
    ? url
        .replace(/^https?:\/\/[^/]+\//, "")
        .split("/")
        .pop() || "Repo"
    : "Repo";
  return {
    title: url ? `Open ${url}` : "Open repository",
    icon: url?.includes("github.com") ? "Github" : "Globe",
    label,
    visible: info.repo && url !== null,
    behavior: { kind: "action", onPress: openRepo },
  };
}

const refIcons = { pr: "GitPullRequest", commit: "GitCommitHorizontal", branch: "GitBranch" };

/** Menu of PRs, commits, and branches named in the agent's last reply; hidden when there are none. */
export function describeRefsPill(
  info: BranchInfo,
  refs: readonly GitRef[],
  open: (url: string) => Promise<void>,
): PluginButton {
  const remote = info.remoteUrl;
  return {
    title: "Git links in the last reply",
    icon: "Link",
    label: `${refs.length} ${refs.length === 1 ? "link" : "links"}`,
    visible: info.repo && remote !== null && refs.length > 0,
    behavior: {
      kind: "menu",
      // Host ids allow only `[a-z][a-z0-9-]*`, so ref values like `feat/x` cannot be ids.
      items: refs.map((ref, index) => ({
        kind: "item" as const,
        id: `ref-${index}`,
        title: refTitle(ref),
        icon: refIcons[ref.kind],
        behavior: {
          kind: "action" as const,
          onPress: () => (remote ? open(refUrl(remote, ref)) : Promise.resolve()),
        },
      })),
    },
  };
}

/** `3 files +14 -3` for uncommitted tracked changes; hidden when the work tree is clean. */
export function describeChangesPill(info: BranchInfo, refresh: () => Promise<void>): PluginButton {
  const changes = info.changes;
  const files = changes ? `${changes.files} ${changes.files === 1 ? "file" : "files"}` : "Changes";
  const lines =
    changes && changes.added !== null && changes.deleted !== null
      ? ` +${changes.added} -${changes.deleted}`
      : "";
  return {
    title: `${files} uncommitted${lines ? `,${lines} lines vs HEAD` : ""} · click to refresh`,
    icon: "FileDiff",
    label: `${files}${lines}`,
    visible: info.repo && info.dirty && changes !== null,
    behavior: { kind: "action", onPress: refresh },
  };
}

/** Pull first: anything behind needs attention before pushing. */
export function syncState(ahead: number, behind: number): SyncState {
  if (behind > 0) return "behind";
  return ahead > 0 ? "ahead" : "synced";
}

/** Green check when in sync, otherwise `↑ahead | ↓behind`; shown with an upstream so a click can fetch. */
export function describeSyncPill(info: BranchInfo, fetch: () => Promise<void>): PluginButton {
  const ahead = info.ahead ?? 0;
  const behind = info.behind ?? 0;
  const state = syncState(ahead, behind);
  const counts = [
    ahead > 0 ? `${ahead} to push` : null,
    behind > 0 ? `${behind} to pull` : null,
  ].filter(Boolean);
  return {
    title: `${counts.join(", ") || "In sync"} vs ${info.upstream ?? "upstream"} · click to fetch`,
    icon: syncIcons[state],
    label: state === "synced" ? "Synced" : `↑${ahead} | ↓${behind}`,
    visible: info.repo && info.upstream !== null,
    behavior: { kind: "action", onPress: fetch },
  };
}
