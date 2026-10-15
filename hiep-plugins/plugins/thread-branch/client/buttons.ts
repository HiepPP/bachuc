import type { PluginButton } from "@getpaseo/plugin/client";
import type { BranchInfo } from "../shared/branch";
import { pillLabel, pillTitle } from "./label";
import { branchIcons, type SyncState } from "./sync-icon";

/** Last path segment of the `origin` web URL, such as `bachuc`. */
export function repoName(url: string) {
  return (
    url
      .replace(/^https?:\/\/[^/]+\//, "")
      .split("/")
      .pop() || "repository"
  );
}

/**
 * `main` when in sync or without an upstream, `main ↑2 ↓1` otherwise (zero counts omitted).
 * The icon turns accent while commits wait to push and amber while commits wait to pull.
 */
export function describeBranchPill(
  info: BranchInfo,
  actions: {
    copy(): Promise<void>;
    openRepo(): Promise<void>;
    fetch(): Promise<void>;
    refresh(): Promise<void>;
  },
): PluginButton {
  const ahead = info.upstream ? (info.ahead ?? 0) : 0;
  const behind = info.upstream ? (info.behind ?? 0) : 0;
  const counts = [ahead > 0 ? `↑${ahead}` : null, behind > 0 ? `↓${behind}` : null].filter(Boolean);
  const url = info.remoteUrl;
  return {
    title: pillTitle(info),
    icon: branchIcons[syncState(ahead, behind)],
    label: [pillLabel(info), ...counts].join(" "),
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
        {
          kind: "item",
          id: "open-repo",
          title: url
            ? `Open ${repoName(url)}${url.includes("github.com") ? " on GitHub" : ""}`
            : "Open repository",
          icon: url?.includes("github.com") ? "Github" : "Globe",
          visible: url !== null,
          behavior: { kind: "action", onPress: actions.openRepo },
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
