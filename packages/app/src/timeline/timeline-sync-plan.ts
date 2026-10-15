import { TIMELINE_FETCH_PAGE_SIZE } from "@/timeline/timeline-fetch-policy";

export interface TimelineSyncCursor {
  epoch: string;
  seq: number;
}

export interface ProjectedTimelineTailFetchPlan {
  direction: "tail";
  limit: number;
  projection: "projected";
}

export interface ProjectedTimelineAfterFetchPlan {
  direction: "after";
  cursor: TimelineSyncCursor;
  limit: number;
  projection: "projected";
}

export interface ProjectedTimelineBeforeFetchPlan {
  direction: "before";
  cursor: TimelineSyncCursor;
  limit: number;
  projection: "projected";
}

export type ProjectedTimelineFetchPlan =
  | ProjectedTimelineTailFetchPlan
  | ProjectedTimelineAfterFetchPlan
  | ProjectedTimelineBeforeFetchPlan;

export type ProjectedTimelineForwardFetchPlan =
  | ProjectedTimelineTailFetchPlan
  | ProjectedTimelineAfterFetchPlan;

export function planTimelineCatchUpAfter(cursor: TimelineSyncCursor) {
  return {
    direction: "after",
    cursor,
    limit: TIMELINE_FETCH_PAGE_SIZE,
    projection: "projected",
  } as const;
}

export function planTimelineTailFetch() {
  return {
    direction: "tail",
    limit: TIMELINE_FETCH_PAGE_SIZE,
    projection: "projected",
  } as const;
}

export function planTimelineResumeFetch(
  range: { epoch: string; endSeq: number } | null | undefined,
): ProjectedTimelineForwardFetchPlan {
  return range
    ? planTimelineCatchUpAfter({ epoch: range.epoch, seq: range.endSeq })
    : planTimelineTailFetch();
}

/**
 * Asks the daemon to widen a tail page back to the latest prompt, so an open that lands on that
 * prompt needs no second fetch. Only tail plans carry it, and only when the host supports it.
 */
export function withLatestPromptTail<Plan extends { direction: "tail" | "after" | "before" }>(
  plan: Plan,
  input: { hostSupportsLatestPromptTail: boolean; latestPromptAnchorPending: boolean },
): Plan | (Plan & { includeLatestPrompt: true }) {
  if (plan.direction !== "tail") return plan;
  if (!input.hostSupportsLatestPromptTail || !input.latestPromptAnchorPending) return plan;
  return { ...plan, includeLatestPrompt: true };
}

export function planTimelineOlderFetch(cursor: TimelineSyncCursor) {
  return {
    direction: "before",
    cursor,
    limit: TIMELINE_FETCH_PAGE_SIZE,
    projection: "projected",
  } as const;
}

export function planTimelinePromptJump(target: TimelineSyncCursor) {
  const newerRows = Math.floor(TIMELINE_FETCH_PAGE_SIZE / 2);
  return {
    direction: "before",
    cursor: { epoch: target.epoch, seq: target.seq + newerRows + 1 },
    limit: TIMELINE_FETCH_PAGE_SIZE,
    projection: "projected",
    mergeWindow: true,
  } as const;
}

export function isTimelineCatchUpComplete(input: {
  direction: "tail" | "before" | "after";
  hasNewer: boolean;
  error: string | null;
}): boolean {
  if (input.error) {
    return false;
  }

  return input.direction !== "after" || !input.hasNewer;
}

export function isTimelineResumeSnapshotAuthoritative(input: {
  direction: "tail" | "before" | "after";
  hasNewer: boolean;
  error: string | null;
}): boolean {
  if (input.error || input.direction === "before") return false;
  return input.direction === "tail" || !input.hasNewer;
}
