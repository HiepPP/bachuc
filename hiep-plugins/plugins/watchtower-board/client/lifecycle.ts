import type { Overview } from "../shared/overview";
import { plural } from "./dashboard";

export const phaseNames = ["Plan", "Implement", "Verify", "Review", "Archive"] as const;
export type PhaseName = (typeof phaseNames)[number];
export type PhaseState = "done" | "current" | "todo";
export interface Phase {
  name: PhaseName;
  state: PhaseState;
  meta: string;
}

export type NeedsYouKind = "question" | "adr" | "checks";
export type NeedsYouTone = "danger" | "warning" | "neutral";
export interface NeedsYouRow {
  kind: NeedsYouKind;
  id: string;
  title: string;
  meta: string;
  tone: NeedsYouTone;
  // Question rows: the default answer, or null when the question has none.
  defaultAnswer?: string | null;
  // Checks row: the text of each pending manual check.
  checks?: string[];
}

type LifecycleInput = Pick<Overview, "plan" | "tasks" | "run" | "planVerify" | "manualChecks">;

// The index of the current phase, per the lifecycle rules in watchtower/CONTEXT.md.
function currentPhase({ plan, tasks, run }: LifecycleInput): number {
  if (plan.status?.toUpperCase() === "ARCHIVED") return 4;
  if (tasks.length === 0) return 0;
  if (run?.finished) return 3;
  if (tasks.every((task) => task.status === "DONE")) return 2;
  return 1;
}

export function lifecyclePhases(overview: LifecycleInput): Phase[] {
  const { tasks, run, planVerify, manualChecks } = overview;
  const current = currentPhase(overview);
  const done = tasks.filter((task) => task.status === "DONE").length;
  const groups = new Set(tasks.map((task) => task.group).filter((group) => group !== null)).size;
  const live = Boolean(run?.started && !run.finished && !run.stopped);
  const runState = live ? ", autorun live" : run?.stopped ? ", run stopped" : "";
  const meta: Record<PhaseName, string> = {
    Plan:
      tasks.length === 0
        ? "No plan yet"
        : groups > 0
          ? `${plural(tasks.length, "task")} in ${plural(groups, "group")}`
          : plural(tasks.length, "task"),
    Implement: `${done} of ${tasks.length} done${runState}`,
    Verify: `${plural(planVerify.length, "check")} and ${plural(manualChecks.length, "manual check")}`,
    Review: run?.finished ? "Draft PR open" : "After the run",
    Archive: "Learnings, then archive",
  };
  return phaseNames.map((name, index) => ({
    name,
    state: index < current ? "done" : index === current ? "current" : "todo",
    meta: meta[name],
  }));
}

type NeedsYouInput = Pick<Overview, "questions" | "decisions" | "manualChecks">;

// Blocking questions first, then other questions, then proposed ADRs, then manual checks.
export function needsYou({ questions, decisions, manualChecks }: NeedsYouInput): NeedsYouRow[] {
  const blocking = (question: NeedsYouInput["questions"][number]) =>
    question.status === "OPEN" && question.blocks.length > 0;
  const rows: NeedsYouRow[] = [];
  // An empty cell, `-`, and `none` all mean the question has no default.
  const realDefault = (question: NeedsYouInput["questions"][number]) =>
    ["", "-", "none"].includes(question.default.trim().toLowerCase()) ? null : question.default;
  for (const question of questions.filter(blocking))
    rows.push({
      kind: "question",
      id: question.id,
      title: question.question,
      meta: `${question.id} blocks ${question.blocks.join(", ")}`,
      tone: "danger",
      defaultAnswer: realDefault(question),
    });
  for (const question of questions.filter((candidate) => !blocking(candidate))) {
    const defaultAnswer = realDefault(question);
    rows.push({
      kind: "question",
      id: question.id,
      title: question.question,
      meta: `${question.id}, ${defaultAnswer ? "has a default" : "open"}`,
      tone: defaultAnswer ? "warning" : "neutral",
      defaultAnswer,
    });
  }
  for (const decision of decisions.filter((row) => row.status.toLowerCase() === "proposed"))
    rows.push({
      kind: "adr",
      id: decision.id,
      title: decision.title,
      meta: `${decision.id}, proposed`,
      tone: "neutral",
    });
  if (manualChecks.length > 0) {
    const tasks = [...new Set(manualChecks.flatMap((check) => check.tasks))];
    rows.push({
      kind: "checks",
      id: "manual-checks",
      title: `${plural(manualChecks.length, "manual check")} pending`,
      meta: tasks.length > 0 ? `From ${tasks.join(", ")}` : "From the Handoff",
      tone: "neutral",
      checks: manualChecks.map((check) => check.text),
    });
  }
  return rows;
}
