import assert from "node:assert/strict";
import test from "node:test";
import { BOARD_LIVE_WINDOW_MS, boardConnectionState } from "../client/connection";

const now = 100_000;

test("paused queries are offline even with cached data or during initial load", () => {
  assert.equal(
    boardConnectionState({
      hasData: true,
      isError: false,
      isPaused: true,
      dataUpdatedAt: now,
      now,
    }),
    "offline",
  );
  assert.equal(
    boardConnectionState({ hasData: false, isError: false, isPaused: true, dataUpdatedAt: 0, now }),
    "offline",
  );
});

test("only a recent successful snapshot is live", () => {
  assert.equal(
    boardConnectionState({
      hasData: false,
      isError: false,
      isPaused: false,
      dataUpdatedAt: 0,
      now,
    }),
    "connecting",
  );
  assert.equal(
    boardConnectionState({
      hasData: true,
      isError: false,
      isPaused: false,
      dataUpdatedAt: now - BOARD_LIVE_WINDOW_MS - 1,
      now,
    }),
    "stale",
  );
  assert.equal(
    boardConnectionState({
      hasData: true,
      isError: false,
      isPaused: false,
      dataUpdatedAt: now - BOARD_LIVE_WINDOW_MS,
      now,
    }),
    "live",
  );
  assert.equal(
    boardConnectionState({
      hasData: true,
      isError: true,
      isPaused: false,
      dataUpdatedAt: now,
      now,
    }),
    "error",
  );
});

test("stars rank first in each column while preserving the existing order within each group", async () => {
  const { starredFirst } = await import("../shared/board");
  for (const column of ["running", "finished"]) {
    const rows = [
      { id: `${column}-first`, starred: false },
      { id: `${column}-star-first`, starred: true },
      { id: `${column}-second`, starred: false },
      { id: `${column}-star-second`, starred: true },
    ];
    assert.deepEqual(
      rows.toSorted(starredFirst).map((row) => row.id),
      [`${column}-star-first`, `${column}-star-second`, `${column}-first`, `${column}-second`],
    );
    rows[1].starred = false;
    assert.deepEqual(
      rows.toSorted(starredFirst).map((row) => row.id),
      [`${column}-star-second`, `${column}-first`, `${column}-star-first`, `${column}-second`],
    );
  }
});
