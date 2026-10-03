import { mkdtemp, mkdir, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { afterEach, describe, expect, test } from "vitest";

import { readCodexOfflineHistory } from "./offline-history.js";

const sessionId = "019f1234-5678-7abc-8def-0123456789ab";
const otherSessionId = "019f1234-5678-7abc-8def-0123456789ac";
const temporaryHomes: string[] = [];

async function makeCodexHome(): Promise<string> {
  const home = await mkdtemp(path.join(os.tmpdir(), "paseo-codex-offline-history-"));
  temporaryHomes.push(home);
  return home;
}

function metadata(id: string, cwd = "/tmp/codex-project") {
  return {
    timestamp: "2026-10-02T01:00:00.000Z",
    type: "session_meta",
    payload: { id, session_id: id, cwd },
  };
}

function event(type: string, payload: Record<string, unknown>, timestamp: string) {
  return { timestamp, type, payload };
}

async function writeTranscript(
  codexHome: string,
  session: string,
  records: unknown[],
  directory = "sessions/2026/10/02",
): Promise<string> {
  const transcriptPath = path.join(
    codexHome,
    directory,
    `rollout-2026-10-02T01-00-00-${session}.jsonl`,
  );
  await mkdir(path.dirname(transcriptPath), { recursive: true });
  await writeFile(
    transcriptPath,
    `${records.map((record) => JSON.stringify(record)).join("\n")}\n`,
  );
  return transcriptPath;
}

afterEach(async () => {
  await Promise.all(
    temporaryHomes.splice(0).map((home) => rm(home, { recursive: true, force: true })),
  );
});

describe("readCodexOfflineHistory", () => {
  test("reads timeline items from the matching session rollout", async () => {
    const codexHome = await makeCodexHome();
    await writeTranscript(codexHome, sessionId, [
      metadata(sessionId),
      event("turn_context", { turn_id: "native-turn-1" }, "2026-10-02T01:00:01.000Z"),
      event(
        "event_msg",
        { type: "user_message", id: "user-item-1", message: "Read this file" },
        "2026-10-02T01:00:02.000Z",
      ),
      event(
        "event_msg",
        { type: "agent_message", id: "assistant-item-1", message: "I will inspect it." },
        "2026-10-02T01:00:03.000Z",
      ),
      event(
        "response_item",
        { type: "reasoning", summary: [{ type: "summary_text", text: "The file is relevant." }] },
        "2026-10-02T01:00:04.000Z",
      ),
      event(
        "event_msg",
        {
          type: "item_completed",
          turn_id: "native-turn-1",
          item: {
            type: "commandExecution",
            id: "command-call-1",
            status: "completed",
            command: ["bash", "-lc", "cat README.md"],
            cwd: "/tmp/codex-project",
            aggregatedOutput: "file contents",
            exitCode: 0,
          },
        },
        "2026-10-02T01:00:05.000Z",
      ),
      event(
        "event_msg",
        { type: "task_complete", turn_id: "native-turn-1" },
        "2026-10-02T01:00:06.000Z",
      ),
    ]);

    const events = await readCodexOfflineHistory({
      sessionId,
      codexHome,
      cwd: "/tmp/codex-project",
    });

    expect(events).toHaveLength(4);
    expect(events.map((entry) => entry.type === "timeline" && entry.item.type)).toEqual([
      "user_message",
      "assistant_message",
      "reasoning",
      "tool_call",
    ]);
    expect(events[0]).toMatchObject({
      type: "timeline",
      provider: "codex",
      turnId: "native-turn-1",
      timestamp: "2026-10-02T01:00:02.000Z",
      item: { type: "user_message", messageId: "user-item-1", text: "Read this file" },
    });
    expect(events[1]).toMatchObject({
      type: "timeline",
      turnId: "native-turn-1",
      timestamp: "2026-10-02T01:00:03.000Z",
      item: {
        type: "assistant_message",
        messageId: "assistant-item-1",
        text: "I will inspect it.",
      },
    });
    expect(events[2]).toMatchObject({
      type: "timeline",
      turnId: "native-turn-1",
      item: { type: "reasoning", text: "The file is relevant." },
    });
    expect(events[3]).toMatchObject({
      type: "timeline",
      turnId: "native-turn-1",
      timestamp: "2026-10-02T01:00:05.000Z",
      item: {
        type: "tool_call",
        callId: "command-call-1",
        status: "completed",
        detail: {
          type: "shell",
          command: "cat README.md",
          output: "file contents",
          exitCode: 0,
        },
      },
    });
  });

  test("pairs native response-item tool calls with outputs and ignores assistant duplicates", async () => {
    const codexHome = await makeCodexHome();
    await writeTranscript(codexHome, sessionId, [
      metadata(sessionId),
      event("turn_context", { turn_id: "native-turn-2" }, "2026-10-02T01:00:01.000Z"),
      event(
        "response_item",
        {
          type: "message",
          id: "response-user-1",
          role: "user",
          content: [{ type: "input_text", text: "Read this file." }],
        },
        "2026-10-02T01:00:01.250Z",
      ),
      event(
        "event_msg",
        { type: "user_message", message: "Read this file." },
        "2026-10-02T01:00:01.500Z",
      ),
      event(
        "response_item",
        {
          type: "function_call",
          call_id: "call-native-1",
          name: "project_index",
          arguments: JSON.stringify({ query: "rollout history" }),
        },
        "2026-10-02T01:00:02.000Z",
      ),
      event(
        "response_item",
        { type: "function_call_output", call_id: "call-native-1", output: "Found two files" },
        "2026-10-02T01:00:03.000Z",
      ),
      event(
        "response_item",
        {
          type: "custom_tool_call",
          call_id: "call-spawn-1",
          name: "spawn_agent",
          input: JSON.stringify({ task: "Inspect the child session.", agent_type: "explorer" }),
        },
        "2026-10-02T01:00:03.500Z",
      ),
      event(
        "response_item",
        {
          type: "custom_tool_call_output",
          call_id: "call-spawn-1",
          output: { thread_id: "child-thread-1", agent_type: "explorer" },
        },
        "2026-10-02T01:00:03.750Z",
      ),
      event(
        "response_item",
        {
          type: "message",
          role: "assistant",
          id: "response-assistant-1",
          content: [{ type: "output_text", text: "Here are the files." }],
        },
        "2026-10-02T01:00:04.000Z",
      ),
      event(
        "event_msg",
        { type: "agent_message", message: "Here are the files." },
        "2026-10-02T01:00:05.000Z",
      ),
    ]);

    const events = await readCodexOfflineHistory({ sessionId, codexHome });

    expect(events).toHaveLength(4);
    expect(events[0]).toMatchObject({
      type: "timeline",
      timestamp: "2026-10-02T01:00:01.500Z",
      item: { type: "user_message", messageId: "response-user-1", text: "Read this file." },
    });
    expect(events[1]).toMatchObject({
      type: "timeline",
      provider: "codex",
      turnId: "native-turn-2",
      timestamp: "2026-10-02T01:00:02.000Z",
      item: {
        type: "tool_call",
        callId: "call-native-1",
        status: "completed",
        detail: {
          type: "unknown",
          input: { query: "rollout history" },
          output: "Found two files",
        },
      },
    });
    expect(events[2]).toMatchObject({
      type: "timeline",
      item: {
        type: "tool_call",
        callId: "call-spawn-1",
        detail: {
          type: "unknown",
          input: { task: "Inspect the child session.", agent_type: "explorer" },
          output: { thread_id: "child-thread-1", agent_type: "explorer" },
        },
      },
    });
    expect(events[3]).toMatchObject({
      type: "timeline",
      item: {
        type: "assistant_message",
        messageId: "response-assistant-1",
        text: "Here are the files.",
      },
    });
  });

  test("removes turns recorded before a legacy rollback marker", async () => {
    const codexHome = await makeCodexHome();
    await writeTranscript(codexHome, sessionId, [
      metadata(sessionId),
      event("turn_context", { turn_id: "turn-one" }, "2026-10-02T01:00:01.000Z"),
      event(
        "event_msg",
        { type: "user_message", message: "Keep this" },
        "2026-10-02T01:00:02.000Z",
      ),
      event(
        "event_msg",
        { type: "agent_message", message: "First answer" },
        "2026-10-02T01:00:03.000Z",
      ),
      event(
        "event_msg",
        { type: "task_complete", turn_id: "turn-one" },
        "2026-10-02T01:00:04.000Z",
      ),
      event("turn_context", { turn_id: "turn-two" }, "2026-10-02T01:00:05.000Z"),
      event(
        "event_msg",
        { type: "user_message", message: "Remove this" },
        "2026-10-02T01:00:06.000Z",
      ),
      event(
        "event_msg",
        { type: "agent_message", message: "Second answer" },
        "2026-10-02T01:00:07.000Z",
      ),
      event("event_msg", { type: "thread_rolled_back", num_turns: 1 }, "2026-10-02T01:00:08.000Z"),
      event("turn_context", { turn_id: "turn-three" }, "2026-10-02T01:00:09.000Z"),
      event(
        "event_msg",
        { type: "user_message", message: "Replacement" },
        "2026-10-02T01:00:10.000Z",
      ),
    ]);

    const events = await readCodexOfflineHistory({ sessionId, codexHome });

    expect(
      events.map((entry) =>
        entry.type === "timeline" && entry.item.type === "user_message" ? entry.item.text : null,
      ),
    ).toEqual(["Keep this", null, "Replacement"]);
    expect(events.map((entry) => ("turnId" in entry ? entry.turnId : undefined))).toEqual([
      "turn-one",
      "turn-one",
      "turn-three",
    ]);
  });

  test("finds archived transcripts by exact session id", async () => {
    const codexHome = await makeCodexHome();
    await writeTranscript(
      codexHome,
      sessionId,
      [
        metadata(sessionId),
        event(
          "event_msg",
          { type: "user_message", message: "Archived" },
          "2026-10-02T01:00:02.000Z",
        ),
      ],
      "archived_sessions/2026/10/02",
    );
    await writeTranscript(codexHome, otherSessionId, [metadata(otherSessionId)]);

    const events = await readCodexOfflineHistory({ sessionId, codexHome });

    expect(events).toHaveLength(1);
    expect(events[0]).toMatchObject({
      type: "timeline",
      item: { type: "user_message", text: "Archived" },
    });
  });

  test("throws clear errors for missing, mismatched, and unsafe histories", async () => {
    const codexHome = await makeCodexHome();

    await expect(readCodexOfflineHistory({ sessionId, codexHome })).rejects.toThrow(
      `No Codex transcript found for session ${sessionId}`,
    );
    await writeTranscript(codexHome, sessionId, [metadata(otherSessionId)]);
    await expect(readCodexOfflineHistory({ sessionId, codexHome })).rejects.toThrow(
      `Codex transcript metadata does not match session ${sessionId}`,
    );
    await expect(
      readCodexOfflineHistory({ sessionId: "../../outside", codexHome }),
    ).rejects.toThrow("Invalid Codex session id");
  });

  test("rejects rollout directories that escape the Codex home", async () => {
    const codexHome = await makeCodexHome();
    const outsideDirectory = await makeCodexHome();
    await symlink(outsideDirectory, path.join(codexHome, "sessions"));

    await expect(readCodexOfflineHistory({ sessionId, codexHome })).rejects.toThrow(
      "Codex sessions path escapes the Codex home",
    );
  });

  test("reports malformed rollout records instead of returning partial history", async () => {
    const codexHome = await makeCodexHome();
    const transcriptPath = await writeTranscript(codexHome, sessionId, [metadata(sessionId)]);
    await writeFile(transcriptPath, `${JSON.stringify(metadata(sessionId))}\n{not json}\n`);

    await expect(readCodexOfflineHistory({ sessionId, codexHome })).rejects.toThrow(
      `Cannot read Codex transcript for session ${sessionId}: Invalid Codex transcript JSON at line 2`,
    );
  });
});
