import assert from "node:assert/strict";
import { test } from "node:test";
import { pillLabel } from "../client/pill-label";
import type { Board, Question, Task } from "../shared/board";

function board(statuses: string[], questions: Question[] = []): Pick<Board, "tasks" | "questions"> {
  return {
    tasks: statuses.map((status, index) => ({ id: `TASK-00${index + 1}`, status }) as Task),
    questions,
  };
}

const question = (id: string): Question => ({ id, question: "Why?", blocks: ["TASK-009"] });

test("no plan gives the plain label", () => {
  assert.equal(pillLabel(undefined), "Watchtower");
  assert.equal(pillLabel(board([])), "Watchtower");
});

test("a plan shows done over total", () => {
  const statuses = [
    "DONE",
    "DONE",
    "DONE",
    "DONE",
    "DONE",
    "IN PROGRESS",
    "TODO",
    "TODO",
    "TODO",
    "BLOCKED",
  ];
  assert.equal(pillLabel(board(statuses)), "Watchtower 5/10");
});

test("blocking questions add a count", () => {
  const statuses = [
    "DONE",
    "DONE",
    "DONE",
    "DONE",
    "DONE",
    "IN PROGRESS",
    "TODO",
    "TODO",
    "TODO",
    "BLOCKED",
  ];
  assert.equal(pillLabel(board(statuses, [question("Q-008")])), "Watchtower 5/10, 1 question");
  assert.equal(
    pillLabel(board(statuses, [question("Q-008"), question("Q-009")])),
    "Watchtower 5/10, 2 questions",
  );
});
