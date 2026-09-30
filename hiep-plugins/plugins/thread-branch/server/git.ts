import { execFile } from "node:child_process";
import type { BranchInfo } from "../shared/branch";

const TIMEOUT = 8_000;
const FETCH_TIMEOUT = 30_000;
const PR_TTL = 5 * 60_000;

export interface Exec {
  code: number | null;
  stdout: string;
  stderr: string;
  enoent: boolean;
}

// Fixed argv only; no shell, so cwd and branch names never reach an interpreter.
export function run(
  file: string,
  args: readonly string[],
  cwd: string,
  timeout = TIMEOUT,
  maxBuffer = 1024 * 1024,
  extraEnv: Readonly<Record<string, string>> = {},
): Promise<Exec> {
  return new Promise((resolve) => {
    execFile(
      file,
      [...args],
      {
        cwd,
        timeout,
        killSignal: "SIGKILL",
        maxBuffer,
        env: {
          ...process.env,
          GIT_TERMINAL_PROMPT: "0",
          GIT_OPTIONAL_LOCKS: "0",
          GH_PROMPT_DISABLED: "1",
          GH_NO_UPDATE_NOTIFIER: "1",
          ...extraEnv,
        },
      },
      (error, stdout, stderr) => {
        const failed = error as
          | (NodeJS.ErrnoException & { code?: unknown; killed?: boolean; signal?: string | null })
          | null;
        // A timeout kill leaves code null with partial output; it must not read as success.
        const code = !failed
          ? 0
          : failed.killed || failed.signal
            ? null
            : typeof failed.code === "number"
              ? failed.code
              : null;
        resolve({
          code,
          stdout: String(stdout ?? ""),
          stderr: String(stderr ?? ""),
          enoent: (error as NodeJS.ErrnoException | null)?.code === "ENOENT",
        });
      },
    );
  });
}

export function parseAheadBehind(output: string): { ahead: number; behind: number } | null {
  const match = output.trim().match(/^(\d+)\s+(\d+)$/);
  if (!match) return null;
  // `rev-list --left-right --count @{upstream}...HEAD` prints "<behind>\t<ahead>".
  return { behind: Number(match[1]), ahead: Number(match[2]) };
}

/** Sums `git diff --numstat` lines; binary files (`-`) add nothing. */
export function sumNumstat(output: string): { added: number; deleted: number } {
  let added = 0;
  let deleted = 0;
  for (const line of output.split("\n")) {
    const match = line.match(/^(-|\d+)\t(-|\d+)\t/);
    if (!match) continue;
    if (match[1] !== "-") added += Number(match[1]);
    if (match[2] !== "-") deleted += Number(match[2]);
  }
  return { added, deleted };
}

export function parsePullRequest(output: string): BranchInfo["pr"] {
  try {
    const value = JSON.parse(output) as { number?: unknown; url?: unknown; state?: unknown };
    if (typeof value.number !== "number" || typeof value.url !== "string") return null;
    return { number: value.number, url: value.url, state: String(value.state ?? "OPEN") };
  } catch {
    return null;
  }
}

/** Turns an `origin` remote into a browser URL: ssh/scp-style and https forms, `.git` stripped. */
export function parseRemoteUrl(output: string): string | null {
  const raw = output.trim();
  if (!raw) return null;
  let host: string;
  let path: string;
  const scp = raw.match(/^(?:[\w.-]+@)?([\w.-]+):(?!\/\/)(.+)$/);
  const url = raw.match(/^(?:https?|ssh|git):\/\/(?:[^@/]+@)?([\w.-]+)(?::\d+)?\/(.+)$/);
  if (url) [host, path] = [url[1], url[2]];
  else if (scp) [host, path] = [scp[1], scp[2]];
  else return null;
  path = path
    .replace(/^\/+/, "")
    .replace(/\.git$/, "")
    .replace(/\/+$/, "");
  if (!path) return null;
  return `https://${host}/${path}`;
}

const empty = (): BranchInfo => ({
  repo: false,
  branch: null,
  detached: false,
  sha: null,
  dirty: false,
  changes: null,
  upstream: null,
  ahead: null,
  behind: null,
  pr: null,
  remoteUrl: null,
  prLookup: "skipped",
});

export function createBranchReader() {
  const prCache = new Map<
    string,
    { at: number; pr: BranchInfo["pr"]; lookup: BranchInfo["prLookup"] }
  >();
  const inflight = new Map<string, Promise<BranchInfo>>();

  async function lookupPullRequest(cwd: string, branch: string, force: boolean) {
    const key = `${cwd}\0${branch}`;
    const cached = prCache.get(key);
    if (!force && cached && Date.now() - cached.at < PR_TTL) return cached;
    const result = await run("gh", ["pr", "view", branch, "--json", "number,url,state"], cwd);
    let entry: { at: number; pr: BranchInfo["pr"]; lookup: BranchInfo["prLookup"] };
    if (result.enoent) entry = { at: Date.now(), pr: null, lookup: "unavailable" };
    else if (result.code === 0) {
      const pr = parsePullRequest(result.stdout);
      entry = { at: Date.now(), pr, lookup: pr ? "ok" : "failed" };
    } else if (/no pull requests found/i.test(result.stderr)) {
      entry = { at: Date.now(), pr: null, lookup: "ok" };
    } else entry = { at: Date.now(), pr: null, lookup: "failed" };
    prCache.set(key, entry);
    return entry;
  }

  async function read(cwd: string, force: boolean, fetch: boolean): Promise<BranchInfo> {
    const inside = await run("git", ["rev-parse", "--is-inside-work-tree"], cwd);
    if (inside.code !== 0 || inside.stdout.trim() !== "true") return empty();
    if (fetch) {
      const fetched = await run("git", ["fetch", "--quiet"], cwd, FETCH_TIMEOUT);
      if (fetched.code !== 0) {
        const reason = fetched.stderr.trim().split("\n").pop() || "timed out or remote unreachable";
        throw new Error(`git fetch failed: ${reason}`);
      }
    }
    const info = { ...empty(), repo: true };
    const symbolic = await run("git", ["symbolic-ref", "--short", "-q", "HEAD"], cwd);
    if (symbolic.code === 0 && symbolic.stdout.trim()) info.branch = symbolic.stdout.trim();
    else info.detached = true;
    const [sha, status, upstream, remote] = await Promise.all([
      run("git", ["rev-parse", "--short", "HEAD"], cwd),
      run("git", ["status", "--porcelain", "--untracked-files=no"], cwd),
      run("git", ["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"], cwd),
      run("git", ["remote", "get-url", "origin"], cwd),
    ]);
    if (sha.code === 0) info.sha = sha.stdout.trim() || null;
    if (remote.code === 0) info.remoteUrl = parseRemoteUrl(remote.stdout);
    info.dirty = status.code === 0 && status.stdout.trim().length > 0;
    if (info.dirty) {
      const diff = await run("git", ["diff", "--numstat", "HEAD", "--"], cwd);
      const lines = diff.code === 0 ? sumNumstat(diff.stdout) : { added: null, deleted: null };
      info.changes = { files: status.stdout.split("\n").filter(Boolean).length, ...lines };
    }
    if (upstream.code === 0 && upstream.stdout.trim()) {
      info.upstream = upstream.stdout.trim();
      const counts = await run(
        "git",
        ["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        cwd,
      );
      const parsed = counts.code === 0 ? parseAheadBehind(counts.stdout) : null;
      if (parsed) {
        info.ahead = parsed.ahead;
        info.behind = parsed.behind;
      }
    }
    if (info.branch) {
      const { pr, lookup } = await lookupPullRequest(cwd, info.branch, force);
      info.pr = pr;
      info.prLookup = lookup;
    }
    return info;
  }

  return {
    get(cwd: string, force = false, fetch = false) {
      const key = `${fetch ? "x" : force ? "f" : "c"}:${cwd}`;
      let pending = inflight.get(key);
      if (!pending) {
        pending = read(cwd, force, fetch).finally(() => inflight.delete(key));
        inflight.set(key, pending);
      }
      return pending;
    },
  };
}
