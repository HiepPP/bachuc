export type GitRefKind = "pr" | "commit" | "branch";

export interface GitRef {
  kind: GitRefKind;
  value: string;
}

export const MAX_REFS = 20;

const PR = /(?:^|[\s([])#(\d{1,6})(?![\w-])/g;
// Hex runs inside UUIDs, hashes in URLs, or longer tokens are not commit SHAs.
const SHA = /(?<![\w\-/.])([0-9a-f]{7,40})(?![\w\-/])/g;
const CODE_SPAN = /`([^`\s]+)`/g;
const BRANCH_NAME = /^(?!.*\.\.)(?!\/)[\w.-]+(?:\/[\w.-]+)*$/;
const BRANCH_PREFIX =
  /^(?:feat|feature|fix|bugfix|hotfix|chore|refactor|docs|test|perf|ci|build|revert|release|codex|claude|paseo)\//;
const BRANCH_WORD = /\bbranch(?:es)?\s*:?\s*$/i;
const DEFAULT_BRANCHES = new Set(["main", "master", "develop"]);

function isSha(value: string) {
  return /\d/.test(value) && /[a-f]/.test(value);
}

/**
 * Finds PR numbers (`#42`), commit SHAs (7–40 hex chars with a digit and a letter), and branch names
 * in an agent reply. Branches must be inline code and either follow the word "branch", use a
 * conventional prefix like `feat/`, or be `main`/`master`/`develop`; file paths stay out.
 */
export function extractGitRefs(text: string): GitRef[] {
  const refs: GitRef[] = [];
  const seen = new Set<string>();
  const add = (kind: GitRefKind, value: string) => {
    const key = `${kind}:${value}`;
    if (seen.has(key) || refs.length >= MAX_REFS) return;
    seen.add(key);
    refs.push({ kind, value });
  };
  const prose = text.replace(/```[\s\S]*?```/g, " ");
  for (const match of prose.matchAll(PR)) add("pr", String(Number(match[1])));
  for (const match of prose.matchAll(SHA)) if (isSha(match[1])) add("commit", match[1]);
  for (const match of prose.matchAll(CODE_SPAN)) {
    const name = match[1];
    if (!BRANCH_NAME.test(name) || /^[0-9a-f]{7,40}$/.test(name)) continue;
    const before = prose.slice(Math.max(0, match.index - 12), match.index);
    if (DEFAULT_BRANCHES.has(name) || BRANCH_PREFIX.test(name) || BRANCH_WORD.test(before))
      add("branch", name);
  }
  return refs;
}

/** Web URL of a ref on the `origin` remote; GitLab uses its `/-/` routes, everything else GitHub's. */
export function refUrl(remoteUrl: string, ref: GitRef) {
  const gitlab = /\/\/[^/]*gitlab/.test(remoteUrl);
  const base = gitlab ? `${remoteUrl}/-` : remoteUrl;
  if (ref.kind === "pr") return `${base}/${gitlab ? "merge_requests" : "pull"}/${ref.value}`;
  if (ref.kind === "commit") return `${base}/commit/${ref.value}`;
  return `${base}/tree/${ref.value.split("/").map(encodeURIComponent).join("/")}`;
}

export function refTitle(ref: GitRef) {
  if (ref.kind === "pr") return `PR #${ref.value}`;
  if (ref.kind === "commit") return `Commit ${ref.value}`;
  return `Branch ${ref.value}`;
}
