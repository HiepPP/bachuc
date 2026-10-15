import type { ProjectedTimelineRow } from "./timeline-projection.js";
import type { AgentTimelineItem } from "./agent-sdk-types.js";
import type { ProviderSubagentDescriptor } from "./provider-subagents/store.js";

export interface AgentTimelineRow {
  seq: number;
  timestamp: string;
  item: AgentTimelineItem;
  readonly turnId?: string;
  readonly providerMessageId?: string;
}

export interface AgentTimelineCursor {
  epoch: string;
  seq: number;
}

export type AgentTimelineFetchDirection = "tail" | "before" | "after";

export interface AgentTimelineFetchOptions {
  direction?: AgentTimelineFetchDirection;
  cursor?: AgentTimelineCursor;
  /**
   * Number of projected items to return.
   * - undefined: store default
   * - 0: all rows in the selected window
   */
  limit?: number;
  /**
   * Tail only: widen the page backward to the latest user message, up to 200 projected
   * items. A prompt further back returns the normal tail.
   */
  includeLatestPrompt?: boolean;
}

export interface AgentTimelineWindow {
  minSeq: number;
  maxSeq: number;
  nextSeq: number;
}

export interface AgentTimelineFetchResult {
  epoch: string;
  direction: AgentTimelineFetchDirection;
  reset: boolean;
  staleCursor: boolean;
  gap: boolean;
  window: AgentTimelineWindow;
  hasOlder: boolean;
  hasNewer: boolean;
  startSeq: number | null;
  endSeq: number | null;
  rows: ProjectedTimelineRow[];
}

export interface DurableTimelineMeta {
  epoch: string;
  nextSeq: number;
  complete: boolean;
}

export interface DurableTimelineSnapshot extends DurableTimelineMeta {
  rows: AgentTimelineRow[];
  providerSubagents: ProviderSubagentDescriptor[];
}

export interface DurableTimelineStore {
  getMeta(agentId: string): DurableTimelineMeta | null;
  /** Flushes this agent's buffer first. Yields every 2,000 rows. */
  load(agentId: string): Promise<DurableTimelineSnapshot | null>;
  /** Deletes rows, writes meta { epoch, nextSeq: 1, complete: false }. Synchronous, one transaction. */
  reset(agentId: string, epoch: string): void;
  /** Buffered. INSERT OR REPLACE by (agent_id, seq). Advances next_seq. */
  append(agentId: string, rows: readonly AgentTimelineRow[]): void;
  /** Buffered. Same statement as append; used for the provider-message-id enrich. */
  update(agentId: string, row: AgentTimelineRow): void;
  /** Buffered; written with the next flush. */
  setProviderSubagents(agentId: string, descriptors: readonly ProviderSubagentDescriptor[]): void;
  /** Flushes the agent's buffer, then sets the flag in the same transaction. */
  setComplete(agentId: string, complete: boolean): void;
  deleteAgent(agentId: string): void;
  flush(): void;
  close(): void;
}
