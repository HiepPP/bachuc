import { execFile } from "node:child_process";
import type { CheckResult, PrCheck, PrNote, PrSnapshot } from "./diff";

const TIMEOUT = 30_000;
const FAILED = new Set([
  "FAILURE",
  "ERROR",
  "CANCELLED",
  "TIMED_OUT",
  "ACTION_REQUIRED",
  "STARTUP_FAILURE",
]);
const PASSED = new Set(["SUCCESS", "NEUTRAL", "SKIPPED"]);
const FIELDS = "number,url,state,mergeable,statusCheckRollup,comments,reviews";

// Fixed argv only; no shell, so the cwd and the PR reference never reach an interpreter.
function runGh(args: readonly string[], cwd: string): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile(
      "gh",
      [...args],
      {
        cwd,
        timeout: TIMEOUT,
        killSignal: "SIGKILL",
        maxBuffer: 8 * 1024 * 1024,
        env: { ...process.env, GH_PROMPT_DISABLED: "1", GH_NO_UPDATE_NOTIFIER: "1" },
      },
      (error, stdout, stderr) => {
        if (error) {
          reject(new Error(stderr.trim() || error.message));
          return;
        }
        resolve(stdout);
      },
    );
  });
}

function record(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

function login(value: unknown): string {
  const author = record(value);
  return typeof author.login === "string" ? author.login : "unknown";
}

function checkResult(raw: Record<string, unknown>): CheckResult {
  const result = String(raw.conclusion || raw.state || "").toUpperCase();
  if (FAILED.has(result)) return "failed";
  return PASSED.has(result) ? "passed" : "pending";
}

/** Parses `gh pr view --json` output into a snapshot. */
export function parsePrView(json: string): PrSnapshot {
  const raw = record(JSON.parse(json));
  if (typeof raw.number !== "number" || typeof raw.url !== "string") {
    throw new Error("gh pr view returned no pull request");
  }
  const state = String(raw.state ?? "OPEN").toUpperCase();
  const checks: PrCheck[] = (Array.isArray(raw.statusCheckRollup) ? raw.statusCheckRollup : []).map(
    (item) => {
      const check = record(item);
      return { name: String(check.name || check.context || "check"), result: checkResult(check) };
    },
  );
  const notes: PrNote[] = [
    ...(Array.isArray(raw.comments) ? raw.comments : []).map((item, index) => {
      const comment = record(item);
      return {
        id: `comment:${String(comment.id ?? comment.url ?? index)}`,
        author: login(comment.author),
        body: String(comment.body ?? ""),
        kind: "comment" as const,
        state: "",
      };
    }),
    ...(Array.isArray(raw.reviews) ? raw.reviews : []).map((item, index) => {
      const review = record(item);
      return {
        id: `review:${String(review.id ?? index)}`,
        author: login(review.author),
        body: String(review.body ?? ""),
        kind: "review" as const,
        state: String(review.state ?? ""),
      };
    }),
  ];
  return {
    number: raw.number,
    url: raw.url,
    state: state === "MERGED" || state === "CLOSED" ? state : "OPEN",
    mergeable: String(raw.mergeable ?? "UNKNOWN").toUpperCase(),
    checks,
    notes,
  };
}

/** Reads one PR. With no number, gh resolves the PR of the branch checked out in `cwd`. */
export async function readPullRequest(cwd: string, number: number | null): Promise<PrSnapshot> {
  const args = ["pr", "view", ...(number === null ? [] : [String(number)]), "--json", FIELDS];
  return parsePrView(await runGh(args, cwd));
}

/** The gh login, so the agent's own comments do not wake it. Null when gh cannot tell. */
export async function readViewerLogin(cwd: string): Promise<string | null> {
  try {
    const value = (await runGh(["api", "user", "--jq", ".login"], cwd)).trim();
    return value || null;
  } catch {
    return null;
  }
}
