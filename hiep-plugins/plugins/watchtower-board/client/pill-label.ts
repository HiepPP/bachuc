import type { Board } from "../shared/board";

// The board RPC returns only OPEN questions that block a task, so each one counts here.
export function pillLabel(board: Pick<Board, "tasks" | "questions"> | null | undefined): string {
  if (!board || board.tasks.length === 0) return "Watchtower";
  const done = board.tasks.filter((task) => task.status === "DONE").length;
  const label = `Watchtower ${done}/${board.tasks.length}`;
  const questions = board.questions.length;
  if (questions === 0) return label;
  return `${label}, ${questions} ${questions === 1 ? "question" : "questions"}`;
}
