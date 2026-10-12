/**
 * @vitest-environment jsdom
 */
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { AccessibilityInfo } from "react-native";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ToolCallDetail } from "@getpaseo/protocol/agent-types";
import type {
  AssistantMessageItem,
  PluginTimelineStreamItem,
  StreamItem,
  ToolCallItem,
} from "@/types/stream";
import { prepareToolCallHistory, projectToolCallDetailLevel } from "../projection";
import type { TurnStepGroup } from "./model";
import {
  areTurnStepsEqual,
  formatTurnClock,
  formatTurnDuration,
  TurnStepListView,
  turnLabelColors,
} from "./view";

// The test translator keeps the key and the count, so the test reads what the view asked for.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { count?: number; duration?: string }) => {
      if (options?.duration !== undefined) return `${key}|${options.duration}`;
      return options?.count === undefined ? key : `${key}#${options.count}`;
    },
  }),
}));

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

function shell(
  id: string,
  second: number,
  status: "running" | "completed" = "completed",
  output?: string,
): ToolCallItem {
  return call(id, second, { type: "shell", command: id, output }, status);
}

function note(id: string, second: number): AssistantMessageItem {
  return { kind: "assistant_message", id, text: id, timestamp: at(second) };
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
    sourceToolCall: call(id, second, { type: "edit", filePath, unifiedDiff: "+a\n+b\n-c" }),
  };
}

const user: StreamItem = { kind: "user_message", id: "user", text: "Go", timestamp: at(0) };

function buildGroup(tail: StreamItem[], isTurnActive = false): TurnStepGroup {
  const result = projectToolCallDetailLevel({
    level: "overview",
    tail,
    head: [],
    preparedHistory: prepareToolCallHistory("overview", tail, { isNonCompact: true }),
    isTurnActive,
    isNonCompact: true,
  });
  const group = [...result.tail].flatMap((item) => {
    const found = result.turnGroupsByHostId.get(item.id);
    return found ? [found] : [];
  })[0];
  if (!group) {
    throw new Error("The sample turn has no group");
  }
  return group;
}

const doneTurn = [
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
  note("final", 12),
];

const runningTurn = [
  user,
  read("r1", 1),
  note("note1", 3),
  shell("dev", 5, "running", "one\ntwo\nthree\nfour"),
];

function noteIds(step: TurnStepGroup["steps"][number]): string[] {
  return step.note.map((block) => block.id);
}

function editIds(step: TurnStepGroup["steps"][number]): string[] {
  return step.edits.map((edit) => edit.id);
}

let root: Root | null = null;
let container: HTMLElement | null = null;

function renderItem(item: { id: string }) {
  return <span data-testid={`item-${item.id}`}>{item.id}</span>;
}

function renderNote(item: { id: string }, isStreaming: boolean) {
  return (
    <p data-testid={`note-${item.id}`} data-streaming={String(isStreaming)}>
      {item.id}
    </p>
  );
}

function mount(group: TurnStepGroup) {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  act(() => {
    root!.render(
      <TurnStepListView group={group} renderItem={renderItem} renderNote={renderNote} />,
    );
  });
}

function all(selector: string): HTMLElement[] {
  return [...container!.querySelectorAll<HTMLElement>(selector)];
}

function one(selector: string): HTMLElement {
  const element = container!.querySelector<HTMLElement>(selector);
  if (!element) {
    throw new Error(`Missing ${selector}`);
  }
  return element;
}

function click(element: HTMLElement) {
  act(() => {
    element.click();
  });
}

beforeEach(() => {
  vi.stubGlobal("React", React);
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
});

afterEach(() => {
  if (root) act(() => root!.unmount());
  container?.remove();
  root = null;
  container = null;
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("TurnStepListView", () => {
  it("draws every note, edit row, and counted tool call of the model once", () => {
    const group = buildGroup(doneTurn);
    mount(group);

    const steps = all('[data-testid^="turn-step-row-"]');
    expect(steps).toHaveLength(group.steps.length);

    const modelNotes = group.steps.flatMap(noteIds);
    expect(all('[data-testid^="note-"]').map((el) => el.textContent)).toEqual(modelNotes);

    const modelEdits = group.steps.flatMap(editIds);
    expect(all('[data-testid^="item-"]').map((el) => el.textContent)).toEqual(modelEdits);

    // A tool call that is not an edit shows in its step meta line as a count.
    const metaText = all('[data-testid^="turn-step-meta-"]')
      .map((el) => el.textContent)
      .join(" ");
    const counted = [...metaText.matchAll(/(\w+)\.(?:one|other)#(\d+)/g)]
      .filter((match) => match[1] !== "editedFiles")
      .reduce((sum, match) => sum + Number(match[2]), 0);
    const modelNonEdit = group.steps.reduce(
      (sum, step) => sum + step.items.length - step.edits.length,
      0,
    );
    expect(counted).toBe(modelNonEdit);

    // Notes + edit rows + counted calls are every non-thought item of the run.
    expect(modelNotes.length + modelEdits.length + counted).toBe(group.run.items.length);
    expect(group.run.items.length).toBe(10);
  });

  it("shows the done header with the duration, with no live box or clock", () => {
    mount(buildGroup(doneTurn));

    expect(one('[data-testid="turn-title"]').textContent).toBe("toolCallGroup.turn.worked|10s");
    expect(container!.querySelector('[data-testid="turn-clock"]')).toBeNull();
    expect(container!.querySelector('[data-testid="turn-live-box"]')).toBeNull();
    const summary = one('[data-testid="turn-summary"]').textContent;
    expect(summary).toContain("toolCallGroup.turn.steps.other#5");
    expect(summary).toContain("toolCallGroup.turn.files.other#2");
    expect(summary).toContain("+4");
    expect(summary).toContain("−2");
    expect(all('[data-testid^="turn-step-time-"]').map((el) => el.textContent)).toEqual([
      "0:03",
      "0:05",
      "0:06",
      "0:08",
      "0:10",
    ]);
  });

  it("highlights a step on a segment click, dims the others, and clears on the second click", () => {
    mount(buildGroup(doneTurn));
    const segments = all('[data-testid^="turn-segment-"]');
    expect(segments).toHaveLength(5);
    expect(all("[data-dimmed='true']")).toHaveLength(0);

    click(segments[2]!);
    expect(one('[data-testid="turn-step-row-2"]').getAttribute("aria-pressed")).toBe("true");
    expect(all('[data-testid^="turn-step-row-"][aria-pressed="true"]')).toHaveLength(1);
    expect(all("[data-testid^='turn-segment-'][data-dimmed='true']")).toHaveLength(4);
    expect(one('[data-testid="turn-segment-2"]').getAttribute("data-dimmed")).not.toBe("true");

    click(one('[data-testid="turn-segment-2"]'));
    expect(all('[data-testid^="turn-step-row-"][aria-pressed="true"]')).toHaveLength(0);
    expect(all("[data-dimmed='true']")).toHaveLength(0);
  });

  it("moves the highlight to a row on a row click", () => {
    mount(buildGroup(doneTurn));

    click(one('[data-testid="turn-step-row-3"]'));
    expect(one('[data-testid="turn-step-row-3"]').getAttribute("aria-pressed")).toBe("true");
    expect(one('[data-testid="turn-segment-3"]').getAttribute("data-dimmed")).not.toBe("true");
    expect(all("[data-testid^='turn-segment-'][data-dimmed='true']")).toHaveLength(4);

    click(one('[data-testid="turn-step-row-3"]'));
    expect(all("[data-dimmed='true']")).toHaveLength(0);
  });

  it("scrolls to a row only when the strip selects it", () => {
    // jsdom has no scrollIntoView.
    const scrollIntoView = vi.fn();
    HTMLElement.prototype.scrollIntoView = scrollIntoView;
    mount(buildGroup(doneTurn));

    click(one('[data-testid="turn-step-row-3"]'));
    expect(scrollIntoView).not.toHaveBeenCalled();
    click(one('[data-testid="turn-step-row-3"]'));

    click(one('[data-testid="turn-segment-1"]'));
    expect(scrollIntoView).toHaveBeenCalledTimes(1);
    delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
  });

  it("keeps edit rows and notes outside the step's button", () => {
    mount(buildGroup(doneTurn));

    expect(one('[data-testid="turn-step-row-0"]').getAttribute("role")).toBe("button");
    expect(one('[data-testid="note-note1"]').closest('[role="button"]')).toBeNull();
    expect(one('[data-testid="item-edit-diffs/e1"]').closest('[role="button"]')).toBeNull();
    // A press in the step body does not toggle the highlight.
    click(one('[data-testid="turn-step-body-0"]'));
    expect(all('[data-testid^="turn-step-row-"][aria-pressed="true"]')).toHaveLength(0);
  });

  it("marks each note with its message id for chat find", () => {
    mount(buildGroup(doneTurn));

    expect(
      one('[data-testid="note-note1"]')
        .closest("[data-message-id]")
        ?.getAttribute("data-message-id"),
    ).toBe("note1");
  });

  it("counts header lines with a line diff, as the edit rows do", () => {
    const edit: PluginTimelineStreamItem = {
      ...editRow("e1", 7, "/repo/calc.py"),
      sourceToolCall: call("e1", 7, {
        type: "edit",
        filePath: "/repo/calc.py",
        oldString: "def add(a, b):\n    return a - b\n",
        newString: "def add(a, b):\n    return a + b\n",
      }),
    };
    mount(buildGroup([user, read("r1", 1), edit, shell("test", 9), note("final", 12)]));

    const summary = one('[data-testid="turn-summary"]').textContent;
    expect(summary).toContain("+1");
    expect(summary).toContain("−1");
  });

  it("listens for reduce motion only on running steps", () => {
    const addEventListener = vi.spyOn(AccessibilityInfo, "addEventListener");
    mount(buildGroup(doneTurn));
    expect(addEventListener).not.toHaveBeenCalled();
    act(() => root!.unmount());
    root = null;

    mount(buildGroup(runningTurn, true));
    // The running step's strip segment and its number.
    expect(addEventListener).toHaveBeenCalledTimes(2);
    addEventListener.mockRestore();
  });

  it("shows the running state: Working, a ticking clock, now, and the live box", () => {
    vi.useFakeTimers({ toFake: ["Date", "setInterval", "clearInterval"] });
    vi.setSystemTime(at(125));
    const group = buildGroup(runningTurn, true);
    mount(group);

    expect(group.steps).toHaveLength(2);
    expect(one('[data-testid="turn-title"]').textContent).toBe("toolCallGroup.turn.working");
    expect(one('[data-testid="turn-clock"]').textContent).toBe("2:05");
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(one('[data-testid="turn-clock"]').textContent).toBe("2:06");

    const times = all('[data-testid^="turn-step-time-"]').map((el) => el.textContent);
    expect(times).toEqual(["0:03", "toolCallGroup.turn.now"]);
    expect(one('[data-testid="turn-summary"]').textContent).toContain("toolCallGroup.turn.step#2");
    expect(one('[data-testid="turn-segment-1"]').getAttribute("data-running")).toBe("true");
    expect(one('[data-testid="turn-segment-0"]').getAttribute("data-running")).not.toBe("true");

    const box = one('[data-testid="turn-live-box"]');
    expect(box.textContent).toContain("$ dev");
    expect(box.textContent).toContain("three");
    expect(box.textContent).toContain("four");
    expect(box.textContent).not.toContain("two");
    expect(all('[data-testid="turn-live-box"]')).toHaveLength(1);
    expect(one('[data-testid="note-note1"]').getAttribute("data-streaming")).toBe("false");
  });
});

describe("turn view helpers", () => {
  it("keeps one fixed palette per label from a single hue table", () => {
    const colors = {
      explore: turnLabelColors("explore"),
      edit: turnLabelColors("edit"),
      verify: turnLabelColors("verify"),
    };
    expect(new Set(Object.values(colors).map((c) => c.segment)).size).toBe(3);
    for (const palette of Object.values(colors)) {
      expect(palette.segment).toMatch(/^rgb\(\d+, \d+, \d+\)$/);
      expect(palette.text).toMatch(/^rgb\(\d+, \d+, \d+\)$/);
      expect(palette.border).toMatch(/^rgb\(\d+, \d+, \d+\)$/);
    }
  });

  it("formats the clock and the duration", () => {
    expect(formatTurnClock(125_000)).toBe("2:05");
    expect(formatTurnClock(0)).toBe("0:00");
    expect(formatTurnDuration(125_000)).toBe("2m 05s");
    expect(formatTurnDuration(45_000)).toBe("45s");
    expect(formatTurnDuration(3_725_000)).toBe("1h 02m");
  });

  it("treats a rebuilt step as equal when no item changed", () => {
    const first = buildGroup(runningTurn, true);
    const second = buildGroup(runningTurn, true);

    expect(first.steps[1]).not.toBe(second.steps[1]);
    expect(areTurnStepsEqual(first.steps[0]!, second.steps[0]!)).toBe(true);
    expect(areTurnStepsEqual(first.steps[1]!, second.steps[1]!)).toBe(true);

    const [, ...unchanged] = runningTurn.slice(0, 3);
    const changed = buildGroup([user, ...unchanged, shell("dev", 5, "running", "five")], true);
    expect(areTurnStepsEqual(first.steps[0]!, changed.steps[0]!)).toBe(true);
    expect(areTurnStepsEqual(first.steps[1]!, changed.steps[1]!)).toBe(false);
  });
});
