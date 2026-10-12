import type { StreamItem } from "@/types/stream";
import type { ToolCallDetailLevel } from "@/hooks/use-settings/storage";
import {
  groupLiveToolCalls,
  groupLiveTurns,
  prepareGroupedHistory,
  prepareTurnHistory,
  type GroupedHistory,
  type GroupedToolCalls,
  type GroupedTurnHistory,
  type ToolCallGroupLookup,
} from "./grouping";
import { buildOverviewGroup, type OverviewToolCallGroup } from "./overview/model";
import { buildTurnGroup, type TurnStepGroup } from "./turn/model";

export type { ToolCallDetailLevel } from "@/hooks/use-settings/storage";
export type ToolCallDetailGroup = OverviewToolCallGroup;

interface PreparedOverviewHistory {
  mode: "overview";
  grouped: GroupedHistory<ToolCallDetailGroup>;
}

interface PreparedTurnHistory {
  mode: "turn";
  grouped: GroupedTurnHistory<TurnStepGroup>;
}

export type PreparedToolCallHistory = PreparedOverviewHistory | PreparedTurnHistory;

export interface ToolCallGroupingOptions {
  /** A non-compact screen groups a whole turn at the overview level. */
  isNonCompact?: boolean;
}

export interface ToolCallHistoryOptions extends ToolCallGroupingOptions {
  /** Turn mode only: an active turn keeps its last note inside the run, so it is no answer yet. */
  isTurnActive?: boolean;
}

export interface ToolCallDetailProjection extends Omit<
  GroupedToolCalls<ToolCallDetailGroup>,
  "historyGroupUpdatesByHostId"
> {
  turnGroupsByHostId: ToolCallGroupLookup<TurnStepGroup>;
  historyGroupUpdatesByHostId: ToolCallGroupLookup<ToolCallDetailGroup | TurnStepGroup>;
}

const EMPTY_TOOL_CALL_GROUPS = new Map<string, ToolCallDetailGroup>();
const EMPTY_TURN_GROUPS = new Map<string, TurnStepGroup>();

// Approval UI owns pending plan presentation. Retain the canonical tool in the
// stream model so resolving it can reveal a card at its original position.
const visibleItemsCache = new WeakMap<StreamItem[], StreamItem[]>();
function visibleToolCallItems(items: StreamItem[]): StreamItem[] {
  const cached = visibleItemsCache.get(items);
  if (cached) return cached;
  const visible = items.filter((item) => {
    if (item.kind !== "tool_call" || item.payload.source !== "agent") return true;
    const data = item.payload.data;
    return (
      data.name !== "ExitPlanMode" && !(data.name === "plan_approval" && data.status === "running")
    );
  });
  const result = visible.length === items.length ? items : visible;
  visibleItemsCache.set(items, result);
  return result;
}

function groupingMode(
  options: ToolCallGroupingOptions | undefined,
): PreparedToolCallHistory["mode"] {
  return options?.isNonCompact ? "turn" : "overview";
}

export function prepareToolCallHistory(
  level: ToolCallDetailLevel,
  tail: StreamItem[],
  options?: ToolCallHistoryOptions,
): PreparedToolCallHistory | null {
  if (level === "detailed") {
    return null;
  }
  if (groupingMode(options) === "turn") {
    return {
      mode: "turn",
      grouped: prepareTurnHistory({
        tail: visibleToolCallItems(tail),
        isTurnActive: options?.isTurnActive ?? false,
        buildGroup: buildTurnGroup,
      }),
    };
  }
  return {
    mode: "overview",
    grouped: prepareGroupedHistory({
      tail: visibleToolCallItems(tail),
      buildGroup: buildOverviewGroup,
    }),
  };
}

export function projectToolCallDetailLevel(
  input: {
    level: ToolCallDetailLevel;
    tail: StreamItem[];
    head: StreamItem[];
    preparedHistory: PreparedToolCallHistory | null;
    isTurnActive: boolean;
  } & ToolCallGroupingOptions,
): ToolCallDetailProjection {
  if (input.level === "detailed") {
    return {
      tail: visibleToolCallItems(input.tail),
      head: visibleToolCallItems(input.head),
      groupsByHostId: EMPTY_TOOL_CALL_GROUPS,
      turnGroupsByHostId: EMPTY_TURN_GROUPS,
      historyGroupUpdatesByHostId: EMPTY_TOOL_CALL_GROUPS,
    };
  }
  const mode = groupingMode(input);
  const prepared = input.preparedHistory;
  if (!prepared || prepared.mode !== mode) {
    throw new Error(`Missing prepared ${mode} tool call history`);
  }
  if (prepared.mode === "turn") {
    const grouped = groupLiveTurns({
      history: prepared.grouped,
      head: visibleToolCallItems(input.head),
      isTurnActive: input.isTurnActive,
      buildGroup: buildTurnGroup,
    });
    return {
      tail: grouped.tail,
      head: grouped.head,
      groupsByHostId: EMPTY_TOOL_CALL_GROUPS,
      turnGroupsByHostId: grouped.groupsByHostId,
      historyGroupUpdatesByHostId: grouped.historyGroupUpdatesByHostId,
    };
  }
  return {
    ...groupLiveToolCalls({
      history: prepared.grouped,
      head: visibleToolCallItems(input.head),
      isTurnActive: input.isTurnActive,
      buildGroup: buildOverviewGroup,
    }),
    turnGroupsByHostId: EMPTY_TURN_GROUPS,
  };
}
