import type { ProjectedTimelineRow } from "./timeline-projection.js";
import type {
  AgentPersistenceHandle,
  AgentSessionConfig,
  AgentTimelineItem,
} from "./agent-sdk-types.js";
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
  /** The provider transcript fingerprint stored with `complete`; absent when none was given. */
  transcriptFingerprint?: string;
}

/**
 * Optional provider hook: a fingerprint of an agent's provider transcript files, built from stat
 * calls only. Null when the files cannot be found or read.
 */
export interface TranscriptFingerprintSource {
  getTranscriptFingerprint(
    handle: AgentPersistenceHandle,
    overrides?: Partial<AgentSessionConfig>,
  ): Promise<string | null>;
}

/** Resume option a TranscriptFingerprintSource reads: the cache already holds this history. */
export interface CachedHistoryResumeOptions {
  historyCached?: boolean;
}

export interface DurableTimelineSnapshot extends DurableTimelineMeta {
  rows: AgentTimelineRow[];
  providerSubagents: ProviderSubagentDescriptor[];
}

/** A projected user or assistant message that may match a search query. */
export interface DurableTimelineSearchCandidate {
  /** Last source seq of the message: the search location. */
  seq: number;
  role: "user" | "assistant";
  /** The projected message text, chunks joined. */
  text: string;
}

/** Called after a flush commits rows of one agent; every buffered row up to `maxSeq` is stored. */
export type TimelineFlushListener = (agentId: string, maxSeq: number) => void;

export interface DurableTimelineStore {
  getMeta(agentId: string): DurableTimelineMeta | null;
  /**
   * Flushes this agent's buffer first. Rows with seq in [fromSeq, toSeq] in seq order, read in
   * pages. Null as in listPromptRows. Optional: without it, memory keeps whole timelines.
   */
  readRowWindow?(agentId: string, fromSeq: number, toSeq: number): AgentTimelineRow[] | null;
  /** Flushes first. Rows below `beforeSeq` whose item identity is listed. Null as above. */
  readIdentityRows?(
    agentId: string,
    identities: readonly string[],
    beforeSeq: number,
  ): AgentTimelineRow[] | null;
  setFlushListener?(listener: TimelineFlushListener | null): void;
  /**
   * Flushes this agent's buffer first. User message rows in seq order. Null when a lost write
   * keeps the agent's rows from serving reads. Optional: without it, reads use memory.
   */
  listPromptRows?(agentId: string): AgentTimelineRow[] | null;
  /**
   * Flushes this agent's buffer first. Messages after `afterSeq` that may contain `query`, in seq
   * order, at most `limit`. A superset of the block-text matches that start at a word. Null as in
   * listPromptRows.
   */
  findSearchCandidates?(
    agentId: string,
    query: string,
    afterSeq: number,
    limit: number,
  ): DurableTimelineSearchCandidate[] | null;
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
  /**
   * Flushes the agent's buffer, then sets the flag in the same transaction. The fingerprint is
   * stored with `complete: true` and cleared otherwise.
   */
  setComplete(agentId: string, complete: boolean, transcriptFingerprint?: string | null): void;
  deleteAgent(agentId: string): void;
  flush(): void;
  close(): void;
}
