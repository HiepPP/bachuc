import type { PaseoApi } from "@getpaseo/client";
import type { PluginAttachmentItem } from "@getpaseo/plugin";
import { parseRemoteUrl, run, type Exec } from "./git";

export type Runner = (
  file: string,
  args: readonly string[],
  cwd: string,
  timeout?: number,
  maxBuffer?: number,
) => Promise<Exec>;

const TTL = 5 * 60_000;
// A run that is still going has no log yet; ask again soon instead of in 5 minutes.
const NOT_READY_TTL = 60_000;
const GH_TIMEOUT = 15_000;
const GH_CONCURRENCY = 3;
const MAX_ITEMS = 20;
export const MAX_BODY = 4_000;
const LOG_LINES = 200;
export const MAX_LOG = 8_000;
// `--log-failed` prints whole job logs; the buffer must hold them to keep the tail.
const LOG_BUFFER = 32 * 1024 * 1024;
const LOG_CACHE_LIMIT = 100;
const FAILED = new Set([
  "FAILURE",
  "ERROR",
  "TIMED_OUT",
  "CANCELLED",
  "ACTION_REQUIRED",
  "STARTUP_FAILURE",
]);
const PASSED = new Set(["SUCCESS", "NEUTRAL", "SKIPPED"]);

interface Check {
  name?: string;
  context?: string;
  conclusion?: string;
  state?: string;
  detailsUrl?: string;
  targetUrl?: string;
}

export interface PullRequest {
  repo: string;
  number: number;
  title: string;
  body: string;
  branch: string;
  author: string;
  url: string;
  updatedAt: string;
  checks: Check[];
}

interface FailedRun {
  runId: string;
  names: string[];
  url: string;
}

/** `owner/repo` for a github.com `origin`, otherwise null. */
export function githubRepo(remote: string): string | null {
  const match = parseRemoteUrl(remote)?.match(/^https:\/\/github\.com\/([^/]+\/[^/]+)$/i);
  return match ? match[1] : null;
}

export function parsePrList(repo: string, output: string): PullRequest[] {
  const value = JSON.parse(output) as unknown;
  if (!Array.isArray(value)) throw new Error("gh pr list returned no array");
  return value.flatMap((raw: Record<string, unknown>) => {
    if (typeof raw?.number !== "number" || typeof raw.url !== "string") return [];
    const author = raw.author as { login?: unknown } | null | undefined;
    return [
      {
        repo,
        number: raw.number,
        title: String(raw.title ?? ""),
        body: String(raw.body ?? ""),
        branch: String(raw.headRefName ?? ""),
        author: typeof author?.login === "string" ? author.login : "unknown",
        url: raw.url,
        updatedAt: String(raw.updatedAt ?? ""),
        checks: Array.isArray(raw.statusCheckRollup) ? (raw.statusCheckRollup as Check[]) : [],
      },
    ];
  });
}

function checkResult(check: Check) {
  const result = (check.conclusion || check.state || "").toUpperCase();
  if (FAILED.has(result)) return "failed";
  return PASSED.has(result) ? "passed" : "pending";
}

const checkName = (check: Check) => check.name || check.context || "check";

export function checkSummary(checks: readonly Check[]) {
  if (checks.length === 0) return "no checks";
  const count = { passed: 0, failed: 0, pending: 0 };
  for (const check of checks) count[checkResult(check)]++;
  const failed = checks.filter((check) => checkResult(check) === "failed").map(checkName);
  return [
    count.passed ? `${count.passed} passed` : null,
    count.failed ? `${count.failed} failed (${failed.join(", ")})` : null,
    count.pending ? `${count.pending} pending` : null,
  ]
    .filter(Boolean)
    .join(", ");
}

/** Failed GitHub Actions checks grouped by workflow run; other CI has no log to fetch. */
export function failedRuns(pr: PullRequest): FailedRun[] {
  const runs = new Map<string, FailedRun>();
  for (const check of pr.checks) {
    if (checkResult(check) !== "failed") continue;
    const runId = (check.detailsUrl || check.targetUrl || "").match(/\/actions\/runs\/(\d+)/)?.[1];
    if (!runId) continue;
    const found = runs.get(runId);
    if (found) found.names.push(checkName(check));
    else
      runs.set(runId, {
        runId,
        names: [checkName(check)],
        url: `https://github.com/${pr.repo}/actions/runs/${runId}`,
      });
  }
  return [...runs.values()];
}

export function cutBody(body: string) {
  return body.length > MAX_BODY ? `${body.slice(0, MAX_BODY)}\n… (cut)` : body;
}

export function tailLog(log: string) {
  const tail = log.trimEnd().split("\n").slice(-LOG_LINES).join("\n");
  return tail.length > MAX_LOG ? tail.slice(-MAX_LOG) : tail;
}

function prText(pr: PullRequest) {
  return [
    `# PR #${pr.number}: ${pr.title}`,
    `Repository: ${pr.repo}`,
    `Branch: ${pr.branch}`,
    `Author: ${pr.author}`,
    `URL: ${pr.url}`,
    `Checks: ${checkSummary(pr.checks)}`,
    "",
    "## Body",
    cutBody(pr.body.trim()) || "(no description)",
  ].join("\n");
}

function ciText(pr: PullRequest, run: FailedRun, log: string) {
  return [
    `# CI failure: PR #${pr.number}: ${pr.title}`,
    `Repository: ${pr.repo}`,
    `Branch: ${pr.branch}`,
    `Failed checks: ${run.names.join(", ")}`,
    `Run: ${run.url}`,
    "",
    `## Failed log (last ${LOG_LINES} lines)`,
    "```",
    log,
    "```",
  ].join("\n");
}

function limiter(max: number) {
  let active = 0;
  const waiting: (() => void)[] = [];
  return async <T>(task: () => Promise<T>): Promise<T> => {
    while (active >= max) await new Promise<void>((resolve) => waiting.push(resolve));
    active++;
    try {
      return await task();
    } finally {
      active--;
      waiting.shift()?.();
    }
  };
}

/** Folders of non-archived Paseo workspaces; the attachment search cannot see the current thread. */
export async function listWorkspaceDirs(paseo: Pick<PaseoApi, "workspaces">) {
  const page = await paseo.workspaces.list({ page: { limit: 200 } });
  return page.entries
    .filter((workspace) => !workspace.archivingAt)
    .map((workspace) => workspace.workspaceDirectory ?? workspace.projectRootPath);
}

interface Repo {
  repo: string;
  cwd: string;
}

export function createPrSearch(
  deps: { run?: Runner; now?: () => number; log?: (line: string) => void } = {},
) {
  const exec = deps.run ?? run;
  const now = deps.now ?? Date.now;
  const log = deps.log ?? ((line: string) => console.warn(`[thread-branch] ${line}`));
  const limit = limiter(GH_CONCURRENCY);
  const gh = (args: readonly string[], cwd: string, maxBuffer?: number) =>
    limit(() => exec("gh", args, cwd, GH_TIMEOUT, maxBuffer));
  let repos: { at: number; value: Promise<Repo[]> } | undefined;
  const lists = new Map<string, { at: number; value: Promise<PullRequest[]> }>();
  const logs = new Map<string, { at: number; text: string | null; notReady?: boolean }>();
  // Warn once per outage, not on every keystroke of the picker.
  let warned = false;
  const warn = (line: string) => {
    if (!warned) log(line);
    warned = true;
  };

  async function discover(listDirs: () => Promise<string[]>): Promise<Repo[]> {
    const dirs = [...new Set(await listDirs())];
    const remotes = await Promise.all(
      dirs.map(async (cwd) => ({
        cwd,
        remote: await exec("git", ["remote", "get-url", "origin"], cwd),
      })),
    );
    const found = new Map<string, Repo>();
    for (const { cwd, remote } of remotes) {
      const repo = remote.code === 0 ? githubRepo(remote.stdout) : null;
      if (repo && !found.has(repo.toLowerCase())) found.set(repo.toLowerCase(), { repo, cwd });
    }
    return [...found.values()];
  }

  function cachedRepos(listDirs: () => Promise<string[]>) {
    if (!repos || now() - repos.at >= TTL) {
      const entry = { at: now(), value: discover(listDirs) };
      entry.value.catch(() => {
        if (repos === entry) repos = undefined;
      });
      repos = entry;
    }
    return repos.value;
  }

  async function listPrs({ repo, cwd }: Repo): Promise<PullRequest[]> {
    const result = await gh(
      [
        "pr",
        "list",
        "--repo",
        repo,
        "--state",
        "open",
        "--limit",
        "20",
        "--json",
        "number,title,body,headRefName,author,url,updatedAt,statusCheckRollup",
      ],
      cwd,
    );
    if (result.enoent) {
      warn("gh not found on the daemon PATH; GitHub PR search returns nothing");
      return [];
    }
    if (result.code !== 0) {
      const reason = result.stderr.trim().split("\n").pop() || "timed out";
      warn(`gh pr list failed for ${repo}: ${reason}`);
      return [];
    }
    try {
      const prs = parsePrList(repo, result.stdout);
      warned = false;
      return prs;
    } catch (error) {
      warn(
        `gh pr list output unreadable for ${repo}: ${error instanceof Error ? error.message : error}`,
      );
      return [];
    }
  }

  function cachedPrs(repo: Repo) {
    const key = repo.repo.toLowerCase();
    const cached = lists.get(key);
    if (cached && now() - cached.at < TTL) return cached.value;
    // Failures resolve to [] and stay cached too, so a missing gh is not retried per keystroke.
    const entry = { at: now(), value: listPrs(repo) };
    lists.set(key, entry);
    return entry.value;
  }

  /** The item text is built at search time with no hook on pick, so logs load for a later search. */
  function fetchLog(repo: Repo, runId: string) {
    const key = `${repo.repo}#${runId}`;
    const cached = logs.get(key);
    if (
      cached &&
      (cached.text === "" || now() - cached.at < (cached.notReady ? NOT_READY_TTL : TTL))
    )
      return;
    if (logs.size >= LOG_CACHE_LIMIT) logs.clear();
    // An empty text marks the fetch in flight.
    logs.set(key, { at: now(), text: "" });
    void gh(["run", "view", runId, "--repo", repo.repo, "--log-failed"], repo.cwd, LOG_BUFFER)
      .then((result) => {
        // gh exits 0 for a running workflow and prints a short notice instead of the log.
        if (
          result.code === 0 &&
          result.stdout.length < 1_000 &&
          /still in progress/i.test(`${result.stdout}\n${result.stderr}`)
        ) {
          logs.set(key, { at: now(), text: null, notReady: true });
          return;
        }
        const text = result.code === 0 && result.stdout.trim() ? tailLog(result.stdout) : null;
        if (text === null) log(`gh run view ${runId} failed for ${repo.repo}`);
        logs.set(key, { at: now(), text });
      })
      .catch(() => logs.set(key, { at: now(), text: null }));
  }

  async function search(query: string, listDirs: () => Promise<string[]>) {
    const found = await cachedRepos(listDirs);
    const prs = (await Promise.all(found.map(cachedPrs)))
      .flat()
      .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
    const words = query.toLowerCase().replace(/#/g, " ").split(/\s+/).filter(Boolean);
    const matches = (text: string) => words.every((word) => text.toLowerCase().includes(word));
    const items: PluginAttachmentItem[] = [];
    for (const pr of prs) {
      if (items.length >= MAX_ITEMS) break;
      const haystack = `${pr.repo} ${pr.number} ${pr.title} ${pr.branch}`;
      if (matches(haystack))
        items.push({
          id: `${pr.repo}#${pr.number}`,
          identifier: `#${pr.number}`,
          title: pr.title,
          subtitle: `${pr.repo} · ${pr.branch} · ${checkSummary(pr.checks)}`,
          url: pr.url,
          resourceType: "GitHub pull request",
          text: prText(pr),
        });
      const repo = found.find((entry) => entry.repo === pr.repo) ?? { repo: pr.repo, cwd: "/" };
      for (const run of failedRuns(pr)) {
        const text = logs.get(`${pr.repo}#${run.runId}`)?.text;
        if (!text) {
          if (matches(haystack)) fetchLog(repo, run.runId);
          continue;
        }
        if (!matches(`${haystack} ci failure ${run.names.join(" ")}`)) continue;
        items.push({
          id: `${pr.repo}#${pr.number}:run-${run.runId}`,
          identifier: `#${pr.number}`,
          title: `CI failure: #${pr.number} ${run.names.join(", ")}`,
          subtitle: `${pr.repo} · ${pr.branch}`,
          url: run.url,
          resourceType: "GitHub CI log",
          text: ciText(pr, run, text),
        });
      }
    }
    return { items: items.slice(0, MAX_ITEMS) };
  }

  return {
    async search(query: string, listDirs: () => Promise<string[]>) {
      try {
        return await search(query, listDirs);
      } catch (error) {
        log(`PR search failed: ${error instanceof Error ? error.message : error}`);
        return { items: [] };
      }
    },
  };
}
