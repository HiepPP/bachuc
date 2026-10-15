import { describe, expect, test } from "vitest";
import { TIMELINE_FETCH_PAGE_SIZE } from "@/timeline/timeline-fetch-policy";
import {
  isTimelineCatchUpComplete,
  isTimelineResumeSnapshotAuthoritative,
  planTimelineCatchUpAfter,
  planTimelineOlderFetch,
  planTimelinePromptJump,
  planTimelineResumeFetch,
  planTimelineTailFetch,
  withLatestPromptTail,
} from "./timeline-sync-plan";

describe("timeline sync planning", () => {
  test("agent synchronization checks a bounded tail page", () => {
    const plan = planTimelineTailFetch();

    expect(plan).toEqual({
      direction: "tail",
      limit: TIMELINE_FETCH_PAGE_SIZE,
      projection: "projected",
    });
  });

  test("a restored canonical range catches up after its exact end", () => {
    expect(planTimelineResumeFetch({ epoch: "epoch-1", endSeq: 49 })).toEqual({
      direction: "after",
      cursor: { epoch: "epoch-1", seq: 49 },
      limit: TIMELINE_FETCH_PAGE_SIZE,
      projection: "projected",
    });
  });

  test("a tail plan asks for the latest prompt only with host support and a pending anchor", () => {
    const tail = planTimelineTailFetch();
    const both = { hostSupportsLatestPromptTail: true, latestPromptAnchorPending: true };

    expect(withLatestPromptTail(tail, both)).toEqual({ ...tail, includeLatestPrompt: true });
    expect(withLatestPromptTail(tail, { ...both, hostSupportsLatestPromptTail: false })).toEqual(
      tail,
    );
    expect(withLatestPromptTail(tail, { ...both, latestPromptAnchorPending: false })).toEqual(tail);
    const after = planTimelineCatchUpAfter({ epoch: "epoch-1", seq: 49 });
    expect(withLatestPromptTail(after, both)).toEqual(after);
  });

  test("older history loads one bounded page before the start cursor", () => {
    const plan = planTimelineOlderFetch({ epoch: "epoch-1", seq: 25 });

    expect(plan).toEqual({
      direction: "before",
      cursor: { epoch: "epoch-1", seq: 25 },
      limit: TIMELINE_FETCH_PAGE_SIZE,
      projection: "projected",
    });
  });

  test("an unloaded prompt jump merges a window with half a page newer than the target", () => {
    expect(planTimelinePromptJump({ epoch: "epoch-1", seq: 42 })).toEqual({
      direction: "before",
      cursor: {
        epoch: "epoch-1",
        seq: 42 + Math.floor(TIMELINE_FETCH_PAGE_SIZE / 2) + 1,
      },
      limit: TIMELINE_FETCH_PAGE_SIZE,
      projection: "projected",
      mergeWindow: true,
    });
  });

  test("catch-up finishes when the daemon reports no newer rows", () => {
    expect(isTimelineCatchUpComplete({ direction: "after", hasNewer: false, error: null })).toBe(
      true,
    );
  });

  test("only tail and completed forward pages carry an authoritative resume snapshot", () => {
    expect(
      isTimelineResumeSnapshotAuthoritative({
        direction: "tail",
        hasNewer: false,
        error: null,
      }),
    ).toBe(true);
    expect(
      isTimelineResumeSnapshotAuthoritative({
        direction: "after",
        hasNewer: false,
        error: null,
      }),
    ).toBe(true);
    expect(
      isTimelineResumeSnapshotAuthoritative({
        direction: "before",
        hasNewer: false,
        error: null,
      }),
    ).toBe(false);
  });
});
