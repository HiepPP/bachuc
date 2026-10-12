import { describe, expect, it } from "vitest";
import type { ToolCallDetail } from "@getpaseo/protocol/agent-types";
import type { StreamItem, ToolCallItem } from "@/types/stream";
import {
  prepareToolCallHistory,
  projectToolCallDetailLevel,
  type PreparedToolCallHistory,
  type ToolCallDetailLevel,
} from "./projection";

type AssistantMessageItem = Extract<StreamItem, { kind: "assistant_message" }>;

function toolCall(
  id: string,
  detail: ToolCallDetail,
  options: {
    name?: string;
    status?: "running" | "completed" | "failed" | "canceled";
  } = {},
): ToolCallItem {
  return {
    kind: "tool_call",
    id,
    timestamp: new Date(`2026-01-01T00:00:${id.padStart(2, "0")}.000Z`),
    payload: {
      source: "agent",
      data: {
        provider: "claude",
        callId: id,
        name: options.name ?? detail.type,
        status: options.status ?? "completed",
        error: options.status === "failed" ? "boom" : null,
        detail,
      },
    },
  };
}

function assistant(id: string): AssistantMessageItem {
  return {
    kind: "assistant_message",
    id,
    text: id,
    timestamp: new Date("2026-01-01T00:01:00.000Z"),
  };
}

function project(input: {
  level: ToolCallDetailLevel;
  tail?: StreamItem[];
  head?: StreamItem[];
  isTurnActive?: boolean;
  preparedHistory?: PreparedToolCallHistory | null;
}) {
  const tail = input.tail ?? [];
  return projectToolCallDetailLevel({
    level: input.level,
    tail,
    head: input.head ?? [],
    preparedHistory: input.preparedHistory ?? prepareToolCallHistory(input.level, tail),
    isTurnActive: input.isTurnActive ?? false,
  });
}

describe("tool call detail-level projection", () => {
  it.each(["detailed", "overview"] as const)(
    "keeps pending approval tools out of %s presentation without removing their canonical position",
    (level) => {
      const pending = toolCall(
        "1",
        { type: "plan", text: "Ship it" },
        { name: "ExitPlanMode", status: "running" },
      );
      const followUp = {
        kind: "user_message",
        id: "question",
        text: "What about tests?",
        timestamp: new Date(2),
      } satisfies StreamItem;
      const tail = [pending, followUp];
      expect(project({ level, tail }).tail).toEqual([followUp]);
      expect(tail).toEqual([pending, followUp]);
      const rejected = toolCall("1", { type: "plan", text: "Ship it" }, { name: "plan_approval" });
      expect(project({ level, tail: [rejected, followUp] }).tail).toEqual([rejected, followUp]);
    },
  );

  it("passes detailed timelines through without grouping work", () => {
    const tail = [toolCall("1", { type: "shell", command: "one" })];
    const head = [toolCall("2", { type: "shell", command: "two" })];

    const prepared = prepareToolCallHistory("detailed", tail);
    const result = project({ level: "detailed", tail, head, preparedHistory: prepared });

    expect(prepared).toBeNull();
    expect(result.tail).toBe(tail);
    expect(result.head).toBe(head);
    expect(result.groupsByHostId.size).toBe(0);
  });

  it("keeps one stable overview host as a run grows", () => {
    const firstCall = toolCall("1", { type: "shell", command: "one" });
    const secondCall = toolCall("2", { type: "read", filePath: "/repo/a.ts" });
    const prepared = prepareToolCallHistory("overview", []);

    const single = project({
      level: "overview",
      head: [firstCall],
      isTurnActive: true,
      preparedHistory: prepared,
    });
    expect(single.head).toEqual([firstCall]);
    expect(single.groupsByHostId.get(firstCall.id)?.run).toMatchObject({
      calls: [firstCall],
      latest: firstCall,
      isSealed: false,
    });

    const grouped = project({
      level: "overview",
      head: [firstCall, secondCall],
      isTurnActive: true,
      preparedHistory: prepared,
    });
    expect(grouped.head).toEqual([
      expect.objectContaining({ id: firstCall.id, timestamp: secondCall.timestamp }),
    ]);
    expect(grouped.groupsByHostId.get(firstCall.id)?.run).toMatchObject({
      calls: [firstCall, secondCall],
      latest: secondCall,
      isSealed: false,
    });
  });

  it("keeps a parallel group loading while any call is still running", () => {
    const calls = [
      toolCall("1", { type: "shell", command: "slow" }, { status: "running" }),
      toolCall("2", { type: "shell", command: "done" }),
    ];
    const result = project({ level: "overview", head: calls, isTurnActive: true });

    expect(result.groupsByHostId.get("1")?.isLoading).toBe(true);
  });

  it("builds a loading aggregate for a one-call run", () => {
    const call = toolCall("1", { type: "shell", command: "one" }, { status: "running" });
    const result = project({ level: "overview", head: [call], isTurnActive: true });
    const group = result.groupsByHostId.get(call.id);
    if (!group) {
      throw new Error("Expected an overview group");
    }

    expect(group).toMatchObject({
      isLoading: true,
      summary: { commandCount: 1 },
    });
  });

  it("keeps an active overview group on its latest call until a visible boundary arrives", () => {
    const calls = [
      toolCall("1", { type: "shell", command: "one" }),
      toolCall("2", { type: "read", filePath: "/repo/a.ts" }),
      toolCall("3", { type: "read", filePath: "/repo/b.ts" }),
      toolCall("4", { type: "edit", filePath: "/repo/a.ts" }),
    ];
    const prepared = prepareToolCallHistory("overview", []);
    const active = project({
      level: "overview",
      head: calls,
      isTurnActive: true,
      preparedHistory: prepared,
    });
    const activeGroup = active.groupsByHostId.get("1");

    expect(activeGroup).toMatchObject({
      mode: "overview",
      run: { id: "1", latest: calls[3], isSealed: false },
    });
    const boundary = assistant("answer");
    const sealed = project({
      level: "overview",
      head: [...calls, boundary],
      isTurnActive: true,
      preparedHistory: prepared,
    });
    expect(sealed.groupsByHostId.get("1")).toMatchObject({
      mode: "overview",
      run: { latest: calls[3], isSealed: true },
      summary: { editedFileCount: 1, readFileCount: 2, commandCount: 1 },
    });
  });

  it("keeps a running overview group live before the agent lifecycle catches up", () => {
    const calls = ["1", "2", "3", "4"].map((id) =>
      toolCall(id, { type: "shell", command: id }, { status: "running" }),
    );

    const result = project({
      level: "overview",
      tail: calls,
      isTurnActive: false,
    });

    expect(result.groupsByHostId.get("1")).toMatchObject({
      run: { latest: calls[3], isSealed: false },
      isLoading: true,
      summary: { commandCount: 4 },
    });
  });

  it("seals the trailing overview group only when the turn ends", () => {
    const calls = ["1", "2", "3", "4"].map((id) => toolCall(id, { type: "shell", command: id }));
    const prepared = prepareToolCallHistory("overview", []);

    const betweenCalls = project({
      level: "overview",
      head: calls,
      isTurnActive: true,
      preparedHistory: prepared,
    });
    const nextCall = toolCall("5", { type: "read", filePath: "/repo/a.ts" });
    const continued = project({
      level: "overview",
      head: [...calls, nextCall],
      isTurnActive: true,
      preparedHistory: prepared,
    });
    const ended = project({
      level: "overview",
      head: [...calls, nextCall],
      isTurnActive: false,
      preparedHistory: prepared,
    });

    expect(betweenCalls.groupsByHostId.get("1")?.run.isSealed).toBe(false);
    expect(continued.groupsByHostId.get("1")?.run).toMatchObject({
      latest: nextCall,
      isSealed: false,
    });
    expect(ended.groupsByHostId.get("1")?.run.isSealed).toBe(true);
  });

  it("builds overview summaries without category-specific presentation data", () => {
    const calls = [
      toolCall("1", { type: "read", filePath: "/repo/src/a.ts" }),
      toolCall("2", { type: "read", filePath: "/repo/src/b.ts" }),
      toolCall("3", { type: "shell", command: "npm test" }),
      toolCall("4", { type: "edit", filePath: "/repo/src/a.ts" }, { status: "failed" }),
    ];

    const overview = project({ level: "overview", head: calls });

    expect(overview.groupsByHostId.get("1")).toEqual({
      mode: "overview",
      run: expect.any(Object),
      isLoading: false,
      summary: {
        editedFileCount: 1,
        commandCount: 1,
        readFileCount: 2,
        searchCount: 0,
        otherToolCount: 0,
        paseoCallCount: 0,
      },
    });
  });

  it("distinguishes reads, searches, and other tools in overview", () => {
    const calls = [
      toolCall("1", { type: "read", filePath: "/repo/src/a.ts" }),
      toolCall("2", { type: "read", filePath: "C:\\repo\\src\\beta.ts" }),
      toolCall("3", { type: "fetch", url: "https://github.com/org/repo" }),
      toolCall(
        "4",
        { type: "search", query: "paseo", toolName: "web_search" },
        { status: "failed" },
      ),
      toolCall("5", { type: "fetch", url: "not a url" }),
    ];

    const result = project({ level: "overview", head: calls });

    expect(result.groupsByHostId.get("1")).toMatchObject({
      summary: {
        editedFileCount: 0,
        commandCount: 0,
        readFileCount: 2,
        searchCount: 1,
        otherToolCount: 2,
      },
    });
  });

  it("counts unique edited files and every shell command in overview", () => {
    const calls = [
      toolCall("1", { type: "edit", filePath: "/repo/a.ts" }),
      toolCall("2", { type: "edit", filePath: "/repo/a.ts" }),
      toolCall("3", { type: "write", filePath: "/repo/b.ts" }),
      toolCall("4", { type: "shell", command: "npm test" }),
      toolCall("5", { type: "shell", command: "npm run lint" }),
      toolCall("6", { type: "read", filePath: "/repo/c.ts" }),
    ];

    const result = project({ level: "overview", head: calls });

    expect(result.groupsByHostId.get("1")).toMatchObject({
      summary: {
        editedFileCount: 2,
        commandCount: 2,
        readFileCount: 1,
        otherToolCount: 0,
      },
    });
  });

  it("counts Paseo calls separately from other tools", () => {
    const calls = [
      toolCall("1", { type: "unknown", input: null, output: null }, { name: "paseo.list_agents" }),
      toolCall(
        "2",
        { type: "unknown", input: null, output: null },
        { name: "mcp__paseo__list_worktrees" },
      ),
      toolCall("3", { type: "fetch", url: "https://paseo.sh" }),
      toolCall("4", { type: "fetch", url: "https://github.com/getpaseo" }),
    ];

    const result = project({ level: "overview", head: calls });

    expect(result.groupsByHostId.get("1")).toMatchObject({
      summary: { otherToolCount: 2, paseoCallCount: 2 },
    });
  });

  it("classifies direct Brave search and Paseo runtime tool names", () => {
    const unknownDetail = { type: "unknown" as const, input: null, output: null };
    const calls = [
      toolCall("1", unknownDetail, { name: "brave-search_brave_web_search" }),
      toolCall("2", unknownDetail, { name: "brave-search_brave_llm_context" }),
      toolCall("3", unknownDetail, { name: "paseo_list_providers" }),
      toolCall("4", unknownDetail, { name: "paseo_list_worktrees" }),
      toolCall("5", unknownDetail, { name: "paseo_list_worktrees" }),
      toolCall("6", unknownDetail, { name: "mcp__exa__web_search" }),
    ];

    const result = project({ level: "overview", head: calls });

    expect(result.groupsByHostId.get("1")).toMatchObject({
      summary: { searchCount: 3, otherToolCount: 0, paseoCallCount: 3 },
    });
  });

  it("reuses prepared history and sealed group models across live-head updates", () => {
    const historicalCalls = ["1", "2", "3", "4"].map((id) =>
      toolCall(id, { type: "shell", command: id }),
    );
    const tail = [...historicalCalls, assistant("boundary")];
    const prepared = prepareToolCallHistory("overview", tail);
    if (!prepared) {
      throw new Error("Overview history must be prepared");
    }
    expect(prepared.grouped.tail).toEqual([
      expect.objectContaining({ id: "1", timestamp: historicalCalls[3]?.timestamp }),
      tail[4],
    ]);
    const first = project({
      level: "overview",
      tail,
      head: [toolCall("5", { type: "read", filePath: "/repo/a.ts" })],
      isTurnActive: true,
      preparedHistory: prepared,
    });
    const second = project({
      level: "overview",
      tail,
      head: [
        toolCall("5", { type: "read", filePath: "/repo/a.ts" }),
        toolCall("6", { type: "read", filePath: "/repo/b.ts" }),
      ],
      isTurnActive: true,
      preparedHistory: prepared,
    });

    expect(first.tail).toBe(prepared.grouped.tail);
    expect(second.tail).toBe(prepared.grouped.tail);
    expect(first.groupsByHostId.get("1")).toBe(prepared.grouped.groupsByHostId.get("1"));
    expect(second.groupsByHostId.get("1")).toBe(prepared.grouped.groupsByHostId.get("1"));
    expect(first.historyGroupUpdatesByHostId.size).toBe(0);
    expect(second.historyGroupUpdatesByHostId).toBe(first.historyGroupUpdatesByHostId);
    expect(second.groupsByHostId.get("5")?.run.calls).toHaveLength(2);
  });

  it("preserves projected history identity during assistant-only head updates", () => {
    const trailingCalls = [
      toolCall("1", { type: "shell", command: "one" }),
      toolCall("2", { type: "read", filePath: "/repo/a.ts" }),
    ];
    const tail = [assistant("before"), ...trailingCalls];
    const prepared = prepareToolCallHistory("overview", tail);
    if (!prepared) {
      throw new Error("Overview history must be prepared");
    }

    const firstHead = [assistant("answer")];
    const secondHead = [{ ...firstHead[0], text: "answer grows" }];
    const first = project({
      level: "overview",
      tail,
      head: firstHead,
      isTurnActive: true,
      preparedHistory: prepared,
    });
    const second = project({
      level: "overview",
      tail,
      head: secondHead,
      isTurnActive: true,
      preparedHistory: prepared,
    });

    expect(first.tail).toBe(prepared.grouped.tail);
    expect(second.tail).toBe(prepared.grouped.tail);
    expect(first.groupsByHostId).toBe(prepared.grouped.groupsByHostId);
    expect(second.groupsByHostId).toBe(prepared.grouped.groupsByHostId);
    expect(first.historyGroupUpdatesByHostId.size).toBe(0);
    expect(second.historyGroupUpdatesByHostId).toBe(first.historyGroupUpdatesByHostId);
  });

  it("forms one group across the retained-history and live-head boundary", () => {
    const tail = [
      assistant("before"),
      toolCall("1", { type: "shell", command: "one" }),
      toolCall("2", { type: "shell", command: "two" }),
    ];
    const head = [
      toolCall("3", { type: "read", filePath: "/repo/a.ts" }),
      toolCall("4", { type: "edit", filePath: "/repo/a.ts" }, { status: "running" }),
    ];

    const result = project({ level: "overview", tail, head, isTurnActive: true });

    expect(result.tail).toEqual([
      tail[0],
      expect.objectContaining({ id: "1", timestamp: tail[2]?.timestamp }),
    ]);
    expect(result.head).toEqual([]);
    expect(result.groupsByHostId.get("1")?.run).toMatchObject({
      calls: [...tail.slice(1), ...head],
      latest: head[1],
      isSealed: false,
    });
    expect(result.historyGroupUpdatesByHostId.get("1")).toBe(result.groupsByHostId.get("1"));
  });

  it("keeps a trailing history-only group in the retained segment", () => {
    const tail = ["1", "2", "3", "4"].map((id) => toolCall(id, { type: "shell", command: id }));

    const result = project({ level: "overview", tail, isTurnActive: false });

    expect(result.tail).toEqual([
      expect.objectContaining({ id: "1", timestamp: tail[3]?.timestamp }),
    ]);
    expect(result.head).toEqual([]);
    expect(result.groupsByHostId.get("1")?.run.isSealed).toBe(true);
  });

  it("hosts single calls while leaving plans and spoken messages ungrouped", () => {
    const singleCall = toolCall("1", { type: "shell", command: "one" });
    const plan = toolCall("2", { type: "plan", text: "Plan" });
    const speak = toolCall(
      "3",
      { type: "unknown", input: "Hello", output: null },
      { name: "speak" },
    );

    const result = project({ level: "overview", head: [singleCall, plan, speak] });

    expect(result.head).toEqual([singleCall, plan, speak]);
    expect(result.groupsByHostId.get(singleCall.id)?.run.calls).toEqual([singleCall]);
    expect(result.groupsByHostId.size).toBe(1);
  });
});

describe("tool call turn mode projection", () => {
  const nonCompact = { isNonCompact: true };
  const user = {
    kind: "user_message",
    id: "user",
    text: "Go",
    timestamp: new Date(0),
  } satisfies StreamItem;

  function block(id: string, index: number): AssistantMessageItem {
    return { ...assistant(`${id}:block:${index}`), blockGroupId: id, blockIndex: index };
  }

  function pluginRow(id: string, sourceToolCall?: ToolCallItem): StreamItem {
    return {
      kind: "plugin",
      id,
      timestamp: new Date(1),
      pluginId: "rows",
      pluginItemId: id,
      itemKind: "row",
      version: 1,
      data: null,
      ...(sourceToolCall ? { sourceToolCall } : {}),
    };
  }

  function projectTurns(input: {
    level?: ToolCallDetailLevel;
    tail?: StreamItem[];
    head?: StreamItem[];
    isTurnActive?: boolean;
    preparedHistory?: PreparedToolCallHistory | null;
  }) {
    const level = input.level ?? "overview";
    const tail = input.tail ?? [];
    const isTurnActive = input.isTurnActive ?? false;
    return projectToolCallDetailLevel({
      level,
      tail,
      head: input.head ?? [],
      preparedHistory:
        input.preparedHistory ??
        prepareToolCallHistory(level, tail, { ...nonCompact, isTurnActive }),
      isTurnActive,
      isNonCompact: true,
    });
  }

  function boundary(kind: "notification" | "todo_list" | "compaction"): StreamItem {
    const timestamp = new Date(2);
    if (kind === "notification") {
      const base = { kind, id: kind, timestamp, sourceType: "notification" as const };
      return { ...base, level: "info", message: "Done" };
    }
    if (kind === "todo_list") {
      const activity = { type: "created" as const, count: 1 };
      return { kind, id: kind, timestamp, provider: "claude", items: [], activity };
    }
    return { kind, id: kind, timestamp, status: "completed" };
  }

  it("keeps today's overview runs on a compact screen", () => {
    const tail = [
      toolCall("1", { type: "shell", command: "one" }),
      assistant("note"),
      toolCall("2", { type: "shell", command: "two" }),
      toolCall("3", { type: "read", filePath: "/repo/a.ts" }),
    ];
    const compact = projectToolCallDetailLevel({
      level: "overview",
      tail,
      head: [],
      preparedHistory: prepareToolCallHistory("overview", tail, { isNonCompact: false }),
      isTurnActive: false,
      isNonCompact: false,
    });

    expect(compact).toEqual(project({ level: "overview", tail }));
    expect(compact.groupsByHostId.size).toBe(2);
    expect(compact.turnGroupsByHostId.size).toBe(0);
  });

  it("keeps detailed rows ungrouped on a non-compact screen", () => {
    const tail = [toolCall("1", { type: "shell", command: "one" }), assistant("note")];
    const result = projectTurns({ level: "detailed", tail });

    expect(prepareToolCallHistory("detailed", tail, nonCompact)).toBeNull();
    expect(result.tail).toBe(tail);
    expect(result.groupsByHostId.size).toBe(0);
    expect(result.turnGroupsByHostId.size).toBe(0);
  });

  it("ends a turn run at a user message and keeps each answer outside", () => {
    const first = toolCall("1", { type: "shell", command: "one" });
    const second = toolCall("2", { type: "shell", command: "two" });
    const tail = [first, assistant("a1"), user, second, assistant("a2")];
    const result = projectTurns({ tail });

    expect(result.tail).toEqual(tail);
    expect(result.turnGroupsByHostId.get("1")?.run.items).toEqual([first]);
    expect(result.turnGroupsByHostId.get("2")?.run.items).toEqual([second]);
  });

  it("joins marked plugin rows and ends the run at an unmarked one", () => {
    const edit = toolCall("2", { type: "edit", filePath: "/repo/a.ts" });
    const first = [
      toolCall("1", { type: "read", filePath: "/repo/a.ts" }),
      pluginRow("diff", edit),
    ];
    const unmarked = pluginRow("panel");
    const second = [toolCall("3", { type: "shell", command: "test" }), assistant("n2")];
    const tail = [...first, assistant("n1"), unmarked, ...second];
    const result = projectTurns({ tail, isTurnActive: true });

    expect(result.tail).toEqual([
      expect.objectContaining({ id: "1" }),
      unmarked,
      expect.objectContaining({ id: "3" }),
    ]);
    expect(result.turnGroupsByHostId.get("1")?.run).toMatchObject({
      items: [...first, assistant("n1")],
      isSealed: true,
    });
    expect(result.turnGroupsByHostId.get("3")?.run).toMatchObject({
      items: second,
      isSealed: false,
    });
  });

  it("keeps the last note inside while the turn runs and outside once it ends", () => {
    const call = toolCall("1", { type: "shell", command: "one" });
    const tail = [user, call, assistant("n1")];
    const prepared = prepareToolCallHistory("overview", tail, nonCompact);

    const running = projectTurns({ tail, isTurnActive: true, preparedHistory: prepared });
    const ended = projectTurns({ tail, isTurnActive: false, preparedHistory: prepared });

    expect(running.tail).toEqual([user, expect.objectContaining({ id: "1" })]);
    expect(running.turnGroupsByHostId.get("1")?.run.items).toEqual([call, assistant("n1")]);
    expect(running.historyGroupUpdatesByHostId.has("1")).toBe(true);
    expect(ended.tail).toBe(prepared?.grouped.tail);
    expect(ended.tail).toEqual([user, call, assistant("n1")]);
    expect(ended.turnGroupsByHostId.get("1")?.run.items).toEqual([call]);
  });

  it("keeps a note that spans history and live head in step", () => {
    const call = toolCall("1", { type: "shell", command: "one" });
    const tail = [user, call, block("m", 0)];
    const prepared = prepareToolCallHistory("overview", tail, nonCompact);

    const first = projectTurns({
      tail,
      head: [block("m", 1)],
      isTurnActive: true,
      preparedHistory: prepared,
    });
    const grown = projectTurns({
      tail,
      head: [{ ...block("m", 1), text: "grown" }],
      isTurnActive: true,
      preparedHistory: prepared,
    });
    const ended = projectTurns({
      tail,
      head: [block("m", 1)],
      isTurnActive: false,
      preparedHistory: prepared,
    });

    expect(first.tail).toEqual([user, expect.objectContaining({ id: "1" })]);
    expect(first.head).toEqual([]);
    expect(first.turnGroupsByHostId.get("1")?.run.items).toEqual([
      call,
      ...tail.slice(2),
      block("m", 1),
    ]);
    expect(grown.tail).toBe(first.tail);
    expect(ended.tail).toBe(prepared?.grouped.tail);
    expect(ended.tail).toEqual([user, call, block("m", 0)]);
    expect(ended.head).toEqual([block("m", 1)]);
    expect(ended.turnGroupsByHostId.get("1")?.run).toMatchObject({ items: [call], isSealed: true });
  });

  it("moves history notes into a run when the live head brings its first tool call", () => {
    const call = toolCall("1", { type: "shell", command: "one" });
    const result = projectTurns({
      tail: [user, assistant("n0")],
      head: [call, assistant("n1")],
      isTurnActive: true,
    });

    expect(result.tail).toEqual([user]);
    expect(result.head).toEqual([expect.objectContaining({ id: "n0", kind: "tool_call" })]);
    expect(result.turnGroupsByHostId.get("n0")?.run.items).toEqual([
      assistant("n0"),
      call,
      assistant("n1"),
    ]);
  });

  const work = [
    toolCall("1", { type: "shell", command: "one" }),
    assistant("n1"),
    toolCall("2", { type: "shell", command: "two" }),
  ];
  const host = expect.objectContaining({ id: "1", kind: "tool_call" });

  it.each([
    ["an unmarked plugin row", pluginRow("panel")],
    ["a notification", boundary("notification")],
  ])("keeps the answer outside a finished turn when %s follows it", (_, after) => {
    const tail = [user, ...work, assistant("final"), after];

    const ended = projectTurns({ tail });
    const running = projectTurns({ tail, isTurnActive: true });

    expect(ended.tail).toEqual([user, host, assistant("final"), after]);
    expect(ended.turnGroupsByHostId.get("1")?.run.items).toEqual(work);
    expect(running.tail).toEqual([user, host, after]);
    expect(running.turnGroupsByHostId.get("1")?.run.items).toEqual([...work, assistant("final")]);
  });

  it("keeps the answer outside when the live head brings it with a plugin row", () => {
    const tail = [user, ...work];
    const head = [assistant("final"), pluginRow("panel")];
    const prepared = prepareToolCallHistory("overview", tail, nonCompact);

    const ended = projectTurns({ tail, head, preparedHistory: prepared });
    const running = projectTurns({ tail, head, isTurnActive: true, preparedHistory: prepared });

    expect(ended.tail).toEqual([user, host]);
    expect(ended.head).toEqual(head);
    expect(ended.turnGroupsByHostId.get("1")?.run.items).toEqual(work);
    expect(running.head).toEqual([pluginRow("panel")]);
    expect(running.turnGroupsByHostId.get("1")?.run.items).toEqual([...work, assistant("final")]);
  });

  it("keeps the answer outside after a user message ends the turn at any later boundary", () => {
    const next = toolCall("3", { type: "shell", command: "three" });
    const tail = [user, ...work, assistant("final"), pluginRow("panel"), user, next];
    const result = projectTurns({ tail, isTurnActive: true });

    expect(result.tail).toEqual([user, host, assistant("final"), pluginRow("panel"), user, next]);
    expect(result.turnGroupsByHostId.get("1")?.run.items).toEqual(work);
    expect(result.turnGroupsByHostId.get("3")?.run.isSealed).toBe(false);
  });

  it.each(["todo_list", "compaction"] as const)(
    "splits a turn at a %s and keeps the note before it inside",
    (kind) => {
      const split = boundary(kind);
      const after = toolCall("3", { type: "shell", command: "three" });
      const tail = [user, ...work, assistant("n2"), split, after, assistant("final")];
      const result = projectTurns({ tail });

      expect(result.tail).toEqual([
        user,
        host,
        split,
        expect.objectContaining({ id: "3" }),
        assistant("final"),
      ]);
      expect(result.turnGroupsByHostId.get("1")?.run.items).toEqual([...work, assistant("n2")]);
      expect(result.turnGroupsByHostId.get("3")?.run.items).toEqual([after]);
    },
  );

  it.each(["todo_list", "compaction"] as const)(
    "keeps the answer outside when a %s ends a finished turn",
    (kind) => {
      const tail = [user, ...work, assistant("final"), boundary(kind)];

      expect(projectTurns({ tail }).turnGroupsByHostId.get("1")?.run.items).toEqual(work);
    },
  );

  it("keeps the answer outside an idle turn whose tool call still shows as running", () => {
    const stale = toolCall("1", { type: "shell", command: "one" }, { status: "running" });
    const tail = [user, stale, assistant("final")];
    const result = projectTurns({ tail });

    expect(result.tail).toEqual([user, host, assistant("final")]);
    expect(result.head).toEqual([]);
    expect(result.turnGroupsByHostId.get("1")?.run).toMatchObject({
      items: [stale],
      isSealed: false,
    });
  });
});
