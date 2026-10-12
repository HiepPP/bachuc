import { describe, expect, it } from "vitest";
import type { ToolCallDetail } from "@getpaseo/protocol/agent-types";
import type {
  AssistantMessageItem,
  PluginTimelineStreamItem,
  StreamItem,
  ToolCallItem,
} from "@/types/stream";
import type { TimelineItemTransform } from "@/plugins/timeline/model";
import { createStreamPresentation } from "@/agent-stream/presentation";
import { prepareToolCallHistory, projectToolCallDetailLevel } from "../projection";
import { collectTurnNoteHosts, type TurnStepGroup } from "./model";

function at(second: number): Date {
  return new Date(Date.UTC(2026, 0, 1, 0, 0, second));
}

function call(
  id: string,
  second: number,
  detail: ToolCallDetail,
  status: "running" | "completed" = "completed",
): ToolCallItem {
  return {
    kind: "tool_call",
    id,
    timestamp: at(second),
    payload: {
      source: "agent",
      data: {
        provider: "claude",
        callId: id,
        name: detail.type,
        status,
        error: null,
        detail,
      },
    },
  };
}

function read(id: string, second: number): ToolCallItem {
  return call(id, second, { type: "read", filePath: `/repo/${id}.ts` });
}

function shell(id: string, second: number, status?: "running" | "completed"): ToolCallItem {
  return call(id, second, { type: "shell", command: id }, status);
}

function note(id: string, second: number, block?: { group: string; index: number }) {
  const item: AssistantMessageItem = {
    kind: "assistant_message",
    id,
    text: id,
    timestamp: at(second),
  };
  return block ? { ...item, blockGroupId: block.group, blockIndex: block.index } : item;
}

function editRow(id: string, second: number, filePath: string): PluginTimelineStreamItem {
  return {
    kind: "plugin",
    id: `edit-diffs/${id}`,
    timestamp: at(second),
    pluginId: "edit-diffs",
    pluginItemId: id,
    itemKind: "edit-diff",
    version: 1,
    data: { filePath },
    sourceToolCall: call(id, second, { type: "edit", filePath }),
  };
}

const user: StreamItem = { kind: "user_message", id: "user", text: "Go", timestamp: at(0) };

function ids(items: readonly StreamItem[]): string[] {
  return items.map((item) => item.id);
}

function projectTurn(tail: StreamItem[], isTurnActive = false) {
  const result = projectToolCallDetailLevel({
    level: "overview",
    tail,
    head: [],
    preparedHistory: prepareToolCallHistory("overview", tail, { isNonCompact: true }),
    isTurnActive,
    isNonCompact: true,
  });
  const groups = [...result.tail, ...result.head].flatMap((item) => {
    const group = result.turnGroupsByHostId.get(item.id);
    return group ? [group] : [];
  });
  return { result, groups };
}

function onlyGroup(tail: StreamItem[], isTurnActive = false) {
  const { groups } = projectTurn(tail, isTurnActive);
  expect(groups).toHaveLength(1);
  return groups[0]!;
}

describe("turn step model", () => {
  const final = note("final", 12);
  const turn = [
    user,
    read("r1", 1),
    shell("s1", 2),
    note("note1", 3),
    read("r2", 4),
    note("note2", 5),
    note("note3", 6),
    editRow("e1", 7, "/repo/a.ts"),
    editRow("e2", 8, "/repo/b.ts"),
    shell("test", 9),
    shell("lint", 10),
    final,
  ];

  it("groups a whole turn into one run of 5 steps with the answer outside it", () => {
    const { result, groups } = projectTurn(turn);
    const group = groups[0]!;

    expect(groups).toHaveLength(1);
    expect(result.tail).toEqual([user, expect.objectContaining({ id: "r1" }), final]);
    expect(result.groupsByHostId.size).toBe(0);
    expect(group.run.items).toEqual(turn.slice(1, -1));
    expect(group.steps.map((step) => ids(step.items))).toEqual([
      ["r1", "s1"],
      ["r2"],
      [],
      ["edit-diffs/e1", "edit-diffs/e2"],
      ["test", "lint"],
    ]);
    expect(group.steps.map((step) => ids(step.note))).toEqual([
      ["note1"],
      ["note2"],
      ["note3"],
      [],
      [],
    ]);
    expect(group.steps[3]?.edits).toEqual([turn[7], turn[8]]);
    expect(group.steps[0]?.counts).toMatchObject({ readFileCount: 1, commandCount: 1 });
    expect(group.steps[3]?.counts).toMatchObject({ editedFileCount: 2 });
  });

  it("labels steps Explore before the first edit, Edit for edits, and Verify after", () => {
    expect(onlyGroup(turn).steps.map((step) => step.label)).toEqual([
      "explore",
      "explore",
      "explore",
      "edit",
      "verify",
    ]);
    const noEdit = onlyGroup([user, read("r1", 1), note("n1", 2), shell("s1", 3), final]);
    expect(noEdit.steps.map((step) => step.label)).toEqual(["explore", "explore"]);
  });

  it("counts Paseo edit rows as edits when edit diffs are off", () => {
    const edit = call("e1", 2, { type: "edit", filePath: "/repo/a.ts" });
    const group = onlyGroup([user, read("r1", 1), edit, note("n1", 3), shell("s1", 4), final]);

    expect(group.steps.map((step) => step.label)).toEqual(["edit", "verify"]);
    expect(group.steps[0]?.edits).toEqual([edit]);
    expect(group.summary.changedFiles).toEqual(["/repo/a.ts"]);
  });

  it("gives each step an end time and a duration from item timestamps", () => {
    const group = onlyGroup(turn);

    expect(group.steps.map((step) => step.endedAt)).toEqual([at(3), at(5), at(6), at(8), at(10)]);
    expect(group.steps.map((step) => step.durationMs)).toEqual([3000, 2000, 1000, 2000, 2000]);
    expect(group.steps[0]?.startedAt).toEqual(at(0));
    expect(group.runningStep).toBeNull();
    expect(group.isLoading).toBe(false);
  });

  it("starts the first step at the turn start, not at a rewritten tool call time", () => {
    // A tool call takes the timestamp of its latest update, so it can lag the turn start.
    const split: StreamItem = {
      kind: "notification",
      sourceType: "notification",
      id: "split",
      timestamp: at(5),
      level: "info",
      message: "Heads up",
    };
    const { groups } = projectTurn([
      user,
      read("r1", 2),
      note("n1", 3),
      split,
      shell("s1", 7),
      note("n2", 8),
      // A tool call keeps n2 a step note: every message after the last tool call is the answer.
      shell("s2", 9),
      final,
    ]);
    const window = onlyGroup([read("r1", 2), note("n1", 3), final]);

    expect(groups.map((group) => group.run.startedAt)).toEqual([at(0), at(5)]);
    expect(groups.map((group) => group.steps[0]?.durationMs)).toEqual([3000, 3000]);
    expect(window.run.startedAt).toEqual(at(2));
  });

  it("labels a non-edit step between two Edit steps as Edit", () => {
    const group = onlyGroup([
      user,
      editRow("e1", 1, "/repo/a.ts"),
      note("n1", 2),
      read("r1", 3),
      note("n2", 4),
      editRow("e2", 5, "/repo/b.ts"),
      note("n3", 6),
      shell("test", 7),
      final,
    ]);

    expect(group.steps.map((step) => step.label)).toEqual(["edit", "edit", "edit", "verify"]);
  });

  it("leaves the running step without an end time", () => {
    const group = onlyGroup([user, read("r1", 1), note("n1", 3), shell("s1", 5, "running")], true);

    expect(group.steps.map((step) => step.isRunning)).toEqual([false, true]);
    expect(group.steps[1]).toMatchObject({ startedAt: at(3), endedAt: null, durationMs: null });
    expect(group.runningStep).toBe(group.steps[1]);
    expect(group.isLoading).toBe(true);
  });

  it("summarizes the step count and the changed file paths", () => {
    const again = editRow("e3", 11, "/repo/a.ts");
    const group = onlyGroup([...turn.slice(0, -1), again, final]);

    expect(group.summary).toEqual({ stepCount: 5, changedFiles: ["/repo/a.ts", "/repo/b.ts"] });
  });

  it("hides thoughts and joins the Markdown blocks of one note", () => {
    const thought: StreamItem = {
      kind: "thought",
      id: "thought",
      text: "Hmm",
      status: "ready",
      timestamp: at(1),
    };
    const blocks = [
      note("n1:block:0", 3, { group: "n1", index: 0 }),
      note("n1:block:1", 3, { group: "n1", index: 1 }),
    ];
    const answer = [
      note("a:block:0", 6, { group: "a", index: 0 }),
      note("a:block:1", 6, { group: "a", index: 1 }),
    ];
    const { result, groups } = projectTurn([
      user,
      thought,
      read("r1", 2),
      ...blocks,
      shell("s1", 4),
      ...answer,
    ]);
    const group = groups[0]!;

    expect(result.tail).toEqual([user, expect.objectContaining({ id: "thought" }), ...answer]);
    expect(group.run.items[0]).toBe(thought);
    expect(group.steps).toHaveLength(2);
    expect(group.steps[0]).toMatchObject({ note: blocks, items: [{ id: "r1" }], startedAt: at(0) });
  });

  it("maps each step note message to the host row of its turn", () => {
    const blocks = [
      note("n1:block:0", 3, { group: "n1", index: 0 }),
      note("n1:block:1", 3, { group: "n1", index: 1 }),
    ];
    const { result } = projectTurn([user, read("r1", 1), ...blocks, shell("s1", 4), final]);

    const hosts = collectTurnNoteHosts(result.tail, result.turnGroupsByHostId);
    expect([...hosts]).toEqual([["n1", "r1"]]);
  });

  it("keeps an answer sent as several messages outside the run", () => {
    const answer = [
      note("a:block:0", 12, { group: "a", index: 0 }),
      note("b:block:0", 13, { group: "b", index: 0 }),
      note("b:block:1", 13, { group: "b", index: 1 }),
    ];
    const { result, groups } = projectTurn([...turn.slice(0, -1), ...answer]);
    const group = groups[0]!;

    expect(result.tail).toEqual([user, expect.objectContaining({ id: "r1" }), ...answer]);
    expect(group.steps.at(-1)?.note).toEqual([]);
    expect(group.run.items).toEqual(turn.slice(1, -1));
  });

  it("builds the 5-step turn through the live head and moves the answer out once it ends", () => {
    const present = createStreamPresentation();
    const tail = turn.slice(0, -1);
    const input = { tail, head: [final], transform: undefined, level: "overview" as const };
    const notes = (group: TurnStepGroup | undefined) =>
      group?.steps.map((step) => ids(step.note)[0] ?? null);

    const running = present({ ...input, isTurnActive: true, isNonCompact: true });
    const ended = present({ ...input, isTurnActive: false, isNonCompact: true });
    const flushed = present({
      ...input,
      tail: turn,
      head: [],
      isTurnActive: false,
      isNonCompact: true,
    });
    const runningGroup = running.turnGroupsByHostId.get("r1");

    expect(running.head).toEqual([]);
    expect(notes(runningGroup)).toEqual([
      "note1:block:0",
      "note2:block:0",
      "note3:block:0",
      null,
      "final:block:0",
    ]);
    expect(runningGroup?.runningStep).toBe(runningGroup?.steps[4]);
    expect(ids(ended.head)).toEqual(["final:block:0"]);
    expect(notes(ended.turnGroupsByHostId.get("r1"))?.at(-1)).toBeNull();
    expect(ended.turnGroupsByHostId.get("r1")?.runningStep).toBeNull();
    expect(ids(flushed.tail).slice(-2)).toEqual(["r1", "final:block:0"]);
    expect(flushed.turnGroupsByHostId.get("r1")?.steps).toHaveLength(5);
  });

  it("regroups history when the turn ends, only in turn mode", () => {
    const done: StreamItem = {
      kind: "notification",
      sourceType: "notification",
      id: "done",
      timestamp: at(13),
      level: "info",
      message: "Done",
    };
    const tail = [user, read("r1", 1), note("n1", 2), shell("s1", 3), final, done];
    const present = createStreamPresentation();
    const input = { tail, head: [], transform: undefined, level: "overview" as const };

    const running = present({ ...input, isTurnActive: true, isNonCompact: true });
    const ended = present({ ...input, isTurnActive: false, isNonCompact: true });
    const compactRunning = present({ ...input, isTurnActive: true });
    const compactEnded = present({ ...input, isTurnActive: false });

    expect(ids(running.tail)).toEqual(["user", "r1", "done"]);
    expect(ids(ended.tail)).toEqual(["user", "r1", "final:block:0", "done"]);
    expect(ended.turnGroupsByHostId.get("r1")?.run.items).toHaveLength(3);
    expect(compactEnded.tail).toBe(compactRunning.tail);
    expect(compactEnded.groupsByHostId).toBe(compactRunning.groupsByHostId);
  });

  it("joins plugin edit rows through the stream presentation on a non-compact screen", () => {
    const edit = call("e1", 2, { type: "edit", filePath: "/repo/a.ts" });
    const transform: TimelineItemTransform = ({ item, sourceId }) =>
      item.type === "tool_call" && item.detail.type === "edit"
        ? [{ type: "plugin", id: sourceId, pluginId: "diffs", kind: "row", version: 1, data: null }]
        : undefined;
    const tail = [user, read("r1", 1), edit, final];
    const present = createStreamPresentation();
    const input = { tail, head: [], transform, level: "overview" as const, isTurnActive: false };

    const wide = present({ ...input, isNonCompact: true });
    const compact = present(input);
    const group = wide.turnGroupsByHostId.get("r1");

    expect(wide.tail.map((item) => item.id)).toEqual(["user", "r1", "final:block:0"]);
    expect(group?.steps[0]?.edits).toMatchObject([{ id: "diffs/e1", sourceToolCall: edit }]);
    expect(compact.turnGroupsByHostId.size).toBe(0);
    expect(compact.tail.map((item) => item.id)).toEqual([
      "user",
      "r1",
      "diffs/e1",
      "final:block:0",
    ]);
  });
});
