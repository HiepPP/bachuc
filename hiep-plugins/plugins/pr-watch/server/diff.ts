export type CheckResult = "passed" | "failed" | "pending";

export interface PrCheck {
  name: string;
  result: CheckResult;
}

export interface PrNote {
  id: string;
  author: string;
  body: string;
  kind: "comment" | "review";
  /** Review state such as CHANGES_REQUESTED; empty for comments. */
  state: string;
}

export interface PrSnapshot {
  number: number;
  url: string;
  state: "OPEN" | "MERGED" | "CLOSED";
  mergeable: string;
  checks: PrCheck[];
  notes: PrNote[];
}

export type PrTrigger =
  | { kind: "checks_failed"; names: string[] }
  | { kind: "checks_passed"; count: number }
  | { kind: "new_notes"; notes: PrNote[] }
  | { kind: "conflict" }
  | { kind: "closed"; state: "MERGED" | "CLOSED" };

const NOTE_EXCERPT = 300;

export function isBotLogin(login: string): boolean {
  return login.endsWith("[bot]");
}

/**
 * What changed between two reads of the same PR. The first read of a watch is a baseline
 * and wakes nobody; the watch tool reports the starting state itself.
 */
export function diffSnapshots(
  previous: PrSnapshot | null,
  next: PrSnapshot,
  viewerLogin: string | null,
): PrTrigger[] {
  if (!previous) return [];
  if (next.state !== "OPEN") return [{ kind: "closed", state: next.state }];

  const triggers: PrTrigger[] = [];
  const failedBefore = new Set(
    previous.checks.filter((check) => check.result === "failed").map((check) => check.name),
  );
  const newlyFailed = next.checks
    .filter((check) => check.result === "failed" && !failedBefore.has(check.name))
    .map((check) => check.name);
  if (newlyFailed.length > 0) triggers.push({ kind: "checks_failed", names: newlyFailed });

  const allPassed = (snapshot: PrSnapshot) =>
    snapshot.checks.length > 0 && snapshot.checks.every((check) => check.result === "passed");
  if (allPassed(next) && !allPassed(previous)) {
    triggers.push({ kind: "checks_passed", count: next.checks.length });
  }

  const seen = new Set(previous.notes.map((note) => note.id));
  const fresh = next.notes.filter(
    (note) => !seen.has(note.id) && note.author !== viewerLogin && !isBotLogin(note.author),
  );
  if (fresh.length > 0) triggers.push({ kind: "new_notes", notes: fresh });

  if (next.mergeable === "CONFLICTING" && previous.mergeable !== "CONFLICTING") {
    triggers.push({ kind: "conflict" });
  }
  return triggers;
}

function excerpt(text: string): string {
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length > NOTE_EXCERPT ? `${flat.slice(0, NOTE_EXCERPT)}…` : flat;
}

export function formatWakePrompt(snapshot: PrSnapshot, triggers: readonly PrTrigger[]): string {
  const lines = [`PR #${snapshot.number} changed: ${snapshot.url}`];
  for (const trigger of triggers) {
    switch (trigger.kind) {
      case "checks_failed":
        lines.push(`- Checks failed: ${trigger.names.join(", ")}.`);
        break;
      case "checks_passed":
        lines.push(`- All ${trigger.count} checks passed.`);
        break;
      case "new_notes":
        for (const note of trigger.notes) {
          const label =
            note.kind === "review" ? `review (${note.state || "COMMENTED"})` : "comment";
          lines.push(`- New ${label} from ${note.author}: ${excerpt(note.body) || "(no text)"}`);
        }
        break;
      case "conflict":
        lines.push("- The PR now has merge conflicts with its base branch.");
        break;
      case "closed":
        lines.push(`- The PR is ${trigger.state.toLowerCase()}. The watch stopped.`);
        break;
    }
  }
  return `<pr-watch>\n${lines.join("\n")}\n</pr-watch>`;
}
