import type { AssistantMessageItem, PluginTimelineStreamItem, ToolCallItem } from "@/types/stream";
import { describeToolCall, turnWorkCall, type TurnRun } from "../grouping";
import { buildOverviewGroup, type OverviewSummary } from "../overview/model";

export type TurnStepLabel = "explore" | "edit" | "verify";

/** A tool call, or the plugin row that replaced one. */
export type TurnStepItem = ToolCallItem | PluginTimelineStreamItem;

export interface TurnStep {
  id: string;
  label: TurnStepLabel;
  /** Markdown blocks of the assistant note that ends the step; empty when it has none. */
  note: readonly AssistantMessageItem[];
  items: readonly TurnStepItem[];
  /** The edit and write rows of `items`, in order. */
  edits: readonly TurnStepItem[];
  counts: OverviewSummary;
  startedAt: Date;
  /** Null while the step runs. */
  endedAt: Date | null;
  durationMs: number | null;
  isRunning: boolean;
}

export interface TurnSummary {
  stepCount: number;
  changedFiles: readonly string[];
}

export interface TurnStepGroup {
  mode: "turn";
  run: TurnRun;
  steps: readonly TurnStep[];
  summary: TurnSummary;
  runningStep: TurnStep | null;
  isLoading: boolean;
}

interface StepDraft {
  id: string;
  note: AssistantMessageItem[];
  items: TurnStepItem[];
  edits: TurnStepItem[];
  /** The step ends at its last item; a note always comes last. */
  lastAt: Date;
}

function editedFilePath(call: ToolCallItem): string | null {
  const { detail } = describeToolCall(call);
  return detail.type === "edit" || detail.type === "write" ? detail.filePath : null;
}

function messageId(item: AssistantMessageItem): string {
  return item.blockGroupId ?? item.id;
}

/**
 * A step is one note plus the tool calls before it. Edits without a note end at the next
 * other tool call, so they form their own Edit step. Thoughts stay hidden.
 */
function draftFor(input: {
  drafts: StepDraft[];
  item: AssistantMessageItem | TurnStepItem;
  startsStep: boolean;
}): StepDraft {
  const { drafts, item } = input;
  if (input.startsStep) {
    drafts.push({ id: item.id, note: [], items: [], edits: [], lastAt: item.timestamp });
  }
  const draft = drafts.at(-1)!;
  draft.lastAt = item.timestamp;
  return draft;
}

function splitSteps(run: TurnRun): StepDraft[] {
  const drafts: StepDraft[] = [];
  for (const item of run.items) {
    const current = drafts.at(-1);
    if (item.kind === "assistant_message") {
      const previous = current?.note.at(-1);
      const isOtherNote = previous !== undefined && messageId(previous) !== messageId(item);
      draftFor({ drafts, item, startsStep: !current || isOtherNote }).note.push(item);
      continue;
    }
    if (item.kind !== "tool_call" && item.kind !== "plugin") {
      continue;
    }
    const call = turnWorkCall(item);
    if (!call) {
      continue;
    }
    const isEdit = editedFilePath(call) !== null;
    const hasNote = current !== undefined && current.note.length > 0;
    const endsEditOnlyStep = !isEdit && current !== undefined && current.edits.length > 0;
    const draft = draftFor({ drafts, item, startsStep: !current || hasNote || endsEditOnlyStep });
    draft.items.push(item);
    if (isEdit) {
      draft.edits.push(item);
    }
  }
  return drafts;
}

function labelStep(index: number, firstEdit: number, lastEdit: number): TurnStepLabel {
  if (firstEdit < 0 || index < firstEdit) {
    return "explore";
  }
  if (index > lastEdit) {
    return "verify";
  }
  return "edit";
}

function countCalls(run: TurnRun, draft: StepDraft): OverviewSummary {
  const calls = draft.items.flatMap((item) => turnWorkCall(item) ?? []);
  return buildOverviewGroup({
    id: run.id,
    calls,
    latest: calls.at(-1) ?? run.latest,
    isSealed: true,
  }).summary;
}

export function buildTurnGroup(run: TurnRun): TurnStepGroup {
  const drafts = splitSteps(run);
  const firstEdit = drafts.findIndex((draft) => draft.edits.length > 0);
  const lastEdit = drafts.findLastIndex((draft) => draft.edits.length > 0);
  let startedAt = run.startedAt;

  const steps = drafts.map((draft, index): TurnStep => {
    const isRunning = !run.isSealed && index === drafts.length - 1;
    const endedAt = isRunning ? null : draft.lastAt;
    const step: TurnStep = {
      id: draft.id,
      label: labelStep(index, firstEdit, lastEdit),
      note: draft.note,
      items: draft.items,
      edits: draft.edits,
      counts: countCalls(run, draft),
      startedAt,
      endedAt,
      durationMs: endedAt ? Math.max(0, endedAt.getTime() - startedAt.getTime()) : null,
      isRunning,
    };
    startedAt = endedAt ?? startedAt;
    return step;
  });

  const changedFiles = new Set<string>();
  for (const step of steps) {
    for (const edit of step.edits) {
      const call = turnWorkCall(edit);
      const filePath = call ? editedFilePath(call) : null;
      if (filePath) {
        changedFiles.add(filePath);
      }
    }
  }

  return {
    mode: "turn",
    run,
    steps,
    summary: { stepCount: steps.length, changedFiles: [...changedFiles] },
    runningStep: steps.find((step) => step.isRunning) ?? null,
    isLoading: !run.isSealed,
  };
}
