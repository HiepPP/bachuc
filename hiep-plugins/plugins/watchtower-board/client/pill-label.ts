import type { Board } from "../shared/board";

// The board RPC returns only OPEN questions that block a task, so each one counts here. `live`
// holds the TASKs whose builder runs now, the same set the plan tree marks; a done TASK is not
// counted.
export function pillLabel(
  board: Pick<Board, "tasks" | "questions"> | null | undefined,
  live: ReadonlySet<string> = new Set(),
): string {
  if (!board || board.tasks.length === 0) return "Watchtower";
  const done = board.tasks.filter((task) => task.status === "DONE").length;
  const running = board.tasks.filter((task) => task.status !== "DONE" && live.has(task.id)).length;
  const parts = [`Watchtower ${done}/${board.tasks.length}`];
  if (running > 0) parts.push(`${running} running`);
  const questions = board.questions.length;
  if (questions > 0) parts.push(`${questions} ${questions === 1 ? "question" : "questions"}`);
  return parts.join(", ");
}
