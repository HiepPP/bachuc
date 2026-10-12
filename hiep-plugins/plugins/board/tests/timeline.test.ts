import assert from "node:assert/strict";
import test from "node:test";
import type { BoardRun } from "../shared/board";
import { timelineSnapshot } from "../server/timeline";
import { BREAK_WIDTH, timelineModel, timelineNotes, type TimelineThread } from "../shared/timeline";

// Local times, so hour ticks and labels match the machine's clock like the app does.
const at = (day: number, hour: number, minute = 0) =>
  new Date(2026, 9, day, hour, minute).getTime();
const NOW = at(12, 12);
const WIDTH = 900;

function thread(
  agentId: string,
  start: number,
  end: number | null,
  overrides: Partial<TimelineThread> = {},
): TimelineThread {
  return {
    agentId,
    title: agentId,
    project: "bachuc",
    projectKey: "project:bachuc",
    status: end === null ? "running" : "completed",
    startedAt: new Date(start).toISOString(),
    endedAt: end === null ? null : new Date(end).toISOString(),
    ...overrides,
  };
}

function run(agentId: string, startedAt: string | null, endedAt: string | null): BoardRun {
  return {
    id: agentId,
    agentId,
    title: agentId,
    starred: false,
    project: "bachuc",
    projectKey: "project:bachuc",
    provider: "claude",
    status: endedAt === null ? "running" : "completed",
    startedAt,
    endedAt,
  };
}

// Half-hour threads every 90 minutes, quiet from 01:30 to 08:10, then a running thread.
function busyDay(): TimelineThread[] {
  const threads: TimelineThread[] = [];
  for (let start = at(11, 13); start <= at(12, 1); start += 90 * 60_000) {
    threads.push(thread(`t${threads.length}`, start, start + 30 * 60_000));
  }
  threads.push(thread("morning-1", at(12, 8, 10), at(12, 8, 40)));
  threads.push(thread("morning-2", at(12, 9, 40), at(12, 10, 10)));
  threads.push(thread("now", at(12, 11, 10), null));
  return threads;
}

test("a 6h 40m idle gap folds into one 48 px break", () => {
  const model = timelineModel([{ hostId: "a", threads: busyDay() }], NOW, WIDTH);
  assert.equal(model.breaks.length, 1);
  assert.equal(model.breaks[0].width, BREAK_WIDTH);
  assert.equal(model.breaks[0].label, "6h 40m");
  assert.equal(model.idleMinutes, 400);
  // No hour tick sits inside the folded gap.
  assert.ok(
    model.ticks.every(
      (tick) => tick.x <= model.breaks[0].x || tick.x >= model.breaks[0].x + BREAK_WIDTH,
    ),
  );
});

test("a 90 minute gap stays at scale", () => {
  const threads = [thread("a", at(11, 12, 30), at(12, 9)), thread("b", at(12, 10, 30), at(12, 12))];
  const model = timelineModel([{ hostId: "a", threads }], NOW, WIDTH);
  assert.deepEqual(model.breaks, []);
});

test("a running thread ends at the now line and its subagents share one row", () => {
  const threads = [
    thread("parent", at(12, 11), null),
    thread("child-1", at(12, 11, 5), at(12, 11, 20), {
      parentAgentId: "parent",
      taskId: "TASK-069",
    }),
    thread("child-2", at(12, 11, 25), null, { parentAgentId: "parent" }),
    thread("grandchild", at(12, 11, 30), at(12, 11, 40), { parentAgentId: "child-2" }),
  ];
  const model = timelineModel([{ hostId: "a", threads }], NOW, WIDTH);
  const rows = model.rows.get("a")!;
  assert.equal(rows.length, 1);
  assert.equal(rows[0].bar.kind, "running");
  assert.ok(Math.abs(rows[0].bar.x + rows[0].bar.width - model.nowX) < 0.001);
  assert.deepEqual(
    rows[0].subagents.map((bar) => bar.agentId),
    ["child-1", "child-2", "grandchild"],
  );
  assert.equal(rows[0].subagents[0].taskId, "TASK-069");
  assert.equal(model.threads, 1);
});

test("rows group by host, then project, and a missing parent makes a top-level row", () => {
  const model = timelineModel(
    [
      {
        hostId: "a",
        threads: [
          thread("late", at(12, 10), at(12, 11), { projectKey: "project:x", project: "x" }),
          thread("orphan", at(12, 9), at(12, 9, 30), { parentAgentId: "gone" }),
          thread("early-x", at(12, 8), at(12, 8, 30), { projectKey: "project:x", project: "x" }),
        ],
      },
      { hostId: "b", threads: [thread("other", at(12, 9), at(12, 10))] },
    ],
    NOW,
    WIDTH,
  );
  assert.deepEqual(
    model.rows.get("a")!.map((row) => row.agentId),
    ["early-x", "late", "orphan"],
  );
  assert.deepEqual(
    model.rows.get("b")!.map((row) => row.agentId),
    ["other"],
  );
});

test("the load strip peaks at the busiest hour and labels it", () => {
  const threads = [
    thread("a", at(12, 9), at(12, 10, 30)),
    thread("b", at(12, 9, 15), at(12, 9, 45)),
    thread("c", at(12, 9, 30), at(12, 9, 40)),
    // A subagent does not count toward agents at once.
    thread("sub", at(12, 9, 20), at(12, 9, 50), { parentAgentId: "b" }),
    thread("d", at(12, 11), null),
  ];
  const model = timelineModel([{ hostId: "a", threads }], NOW, WIDTH);
  assert.deepEqual(model.peak, { count: 3, label: "09:00–10:00" });
  assert.equal(Math.max(...model.load.map((bucket) => bucket.count)), 3);
  assert.ok(model.load.some((bucket) => bucket.running));
  assert.equal(model.threads, 4);
});

test("a thread with no start time is left out and counted", () => {
  const now = Date.parse("2026-10-12T12:00:00.000Z");
  const result = timelineSnapshot(
    [
      run("kept", "2026-10-12T10:00:00.000Z", "2026-10-12T11:00:00.000Z"),
      run("running", "2026-10-12T11:30:00.000Z", null),
      run("no-start", null, "2026-10-12T11:00:00.000Z"),
      run("old", "2026-10-10T10:00:00.000Z", "2026-10-10T11:00:00.000Z"),
      run("spans-window", "2026-10-11T10:00:00.000Z", "2026-10-11T13:00:00.000Z"),
    ],
    null,
    now,
  );
  assert.deepEqual(
    result.threads.map((item) => item.agentId),
    ["kept", "running", "spans-window"],
  );
  assert.equal(result.missingStart, 1);
  assert.equal(result.olderDropped, false);
  assert.deepEqual(timelineNotes(result, 3), ["1 thread without a start time is left out."]);
});

test("a full store that dropped runs inside the window says so", () => {
  const now = Date.parse("2026-10-12T12:00:00.000Z");
  assert.equal(timelineSnapshot([], "2026-10-12T02:00:00.000Z", now).olderDropped, true);
  assert.equal(timelineSnapshot([], "2026-10-10T02:00:00.000Z", now).olderDropped, false);
  assert.deepEqual(timelineNotes({ missingStart: 0, olderDropped: true }, 2), [
    "Older threads are not kept.",
  ]);
});

test("no threads gives an empty chart and the nothing-ran note", () => {
  const model = timelineModel([{ hostId: "a", threads: [] }], NOW, WIDTH);
  assert.equal(model.threads, 0);
  assert.equal(model.peak, null);
  assert.deepEqual(model.load, []);
  // The quiet day stays at scale instead of folding into one 24h break.
  assert.deepEqual(model.breaks, []);
  assert.equal(model.idleMinutes, 0);
  assert.deepEqual(timelineNotes({ missingStart: 0, olderDropped: false }, 0), [
    "Nothing ran in the last 24 hours.",
  ]);
});

test("timing: the timeline for 50 runs answers in under 30 ms", () => {
  const now = NOW;
  const runs: BoardRun[] = [];
  for (let index = 0; index < 50; index++) {
    const start = now - (index + 1) * 25 * 60_000;
    const item = run(
      `r${index}`,
      new Date(start).toISOString(),
      index % 10 === 0 ? null : new Date(start + 20 * 60_000).toISOString(),
    );
    runs.push(index % 3 === 1 ? { ...item, parentAgentId: `r${index - 1}` } : item);
  }
  const begin = performance.now();
  const snapshot = timelineSnapshot(runs, runs.at(-1)!.endedAt, now);
  const model = timelineModel([{ hostId: "a", threads: snapshot.threads }], now, WIDTH);
  const elapsed = performance.now() - begin;
  console.log(`timeline 50 runs: ${elapsed.toFixed(2)} ms`);
  assert.equal(snapshot.threads.length, 50);
  assert.ok(model.threads > 0);
  assert.ok(elapsed < 30, `took ${elapsed} ms`);
});
