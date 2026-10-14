import type { AgentBackgroundProcess } from "@getpaseo/protocol/messages";
import type { ToolCallGroupLookup } from "@/tool-calls/detail-level/grouping";
import type { TurnStepGroup } from "@/tool-calls/detail-level/turn/model";
import { isAgentToolCallItem, type StreamItem } from "@/types/stream";

export interface BackgroundProcessHostSlot {
  processes: AgentBackgroundProcess[];
  /** Running processes this turn started that now show in a later turn. */
  movedCount: number;
}

export interface BackgroundProcessPlacement {
  byHostId: ReadonlyMap<string, BackgroundProcessHostSlot>;
  /** Running processes with no step card in the newest turn; the stream bottom shows them. */
  unplaced: AgentBackgroundProcess[];
}

export const EMPTY_BACKGROUND_PROCESS_PLACEMENT: BackgroundProcessPlacement = {
  byHostId: new Map(),
  unplaced: [],
};

function slotFor(
  byHostId: Map<string, BackgroundProcessHostSlot>,
  hostId: string,
): BackgroundProcessHostSlot {
  let slot = byHostId.get(hostId);
  if (!slot) {
    slot = { processes: [], movedCount: 0 };
    byHostId.set(hostId, slot);
  }
  return slot;
}

/**
 * A running process shows under the step card of the newest turn, so it stays in view while the
 * user keeps talking. A finished one goes back to the turn whose tool call started it. A finished
 * process whose turn is not loaded or has no step card is not shown.
 */
export function placeBackgroundProcesses(input: {
  rows: readonly StreamItem[];
  groupsByHostId: ToolCallGroupLookup<TurnStepGroup>;
  processes: readonly AgentBackgroundProcess[];
}): BackgroundProcessPlacement {
  if (input.processes.length === 0) return EMPTY_BACKGROUND_PROCESS_PLACEMENT;

  const hostIdByCallId = new Map<string, string>();
  let newestHostId: string | null = null;
  for (const row of input.rows) {
    if (row.kind === "user_message") {
      newestHostId = null;
      continue;
    }
    const group = input.groupsByHostId.get(row.id);
    if (!group) continue;
    newestHostId = row.id;
    for (const item of group.run.items) {
      if (isAgentToolCallItem(item)) hostIdByCallId.set(item.payload.data.callId, row.id);
    }
  }

  const byHostId = new Map<string, BackgroundProcessHostSlot>();
  const unplaced: AgentBackgroundProcess[] = [];
  for (const process of input.processes) {
    const originHostId = process.toolCallId ? hostIdByCallId.get(process.toolCallId) : undefined;
    if (process.status !== "running") {
      if (originHostId) slotFor(byHostId, originHostId).processes.push(process);
      continue;
    }
    if (newestHostId) {
      slotFor(byHostId, newestHostId).processes.push(process);
    } else {
      unplaced.push(process);
    }
    if (originHostId && originHostId !== newestHostId) {
      slotFor(byHostId, originHostId).movedCount += 1;
    }
  }
  return { byHostId, unplaced };
}
