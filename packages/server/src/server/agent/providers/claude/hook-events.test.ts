import { describe, expect, it } from "vitest";
import {
  hookToolName,
  readClaudeHookMessage,
  slashCommandSkill,
  truncateHookOutput,
} from "./hook-events.js";

describe("readClaudeHookMessage", () => {
  it("reads a started hook", () => {
    expect(
      readClaudeHookMessage({
        type: "system",
        subtype: "hook_started",
        hook_id: "h1",
        hook_name: "PreToolUse:Bash",
        hook_event: "PreToolUse",
        uuid: "u1",
        session_id: "s1",
      }),
    ).toEqual({ phase: "started", id: "h1", name: "PreToolUse:Bash", event: "PreToolUse" });
  });

  it("reads a failed hook response with its exit code and output", () => {
    expect(
      readClaudeHookMessage({
        type: "system",
        subtype: "hook_response",
        hook_id: "h1",
        hook_name: "PreToolUse:Bash",
        hook_event: "PreToolUse",
        output: "blocked",
        stdout: "",
        stderr: "blocked",
        exit_code: 2,
        outcome: "error",
        uuid: "u2",
        session_id: "s1",
      }),
    ).toEqual({
      phase: "response",
      id: "h1",
      name: "PreToolUse:Bash",
      event: "PreToolUse",
      status: "error",
      exitCode: 2,
      output: "blocked",
    });
  });

  it("ignores other system messages", () => {
    expect(readClaudeHookMessage({ type: "system", subtype: "status" })).toBeNull();
  });
});

describe("hookToolName", () => {
  it("returns the tool of a tool hook", () => {
    expect(hookToolName("PreToolUse:Bash")).toBe("Bash");
    expect(hookToolName("PostToolUse:Grep")).toBe("Grep");
    expect(hookToolName("PostToolUseFailure:Edit")).toBe("Edit");
  });

  it("returns null for a hook that has no tool", () => {
    expect(hookToolName("SessionStart:startup")).toBeNull();
    expect(hookToolName("UserPromptSubmit")).toBeNull();
    expect(hookToolName("Stop")).toBeNull();
  });
});

describe("truncateHookOutput", () => {
  it("keeps short output", () => {
    expect(truncateHookOutput("ok")).toBe("ok");
  });

  it("cuts long output to 2048 characters", () => {
    expect(truncateHookOutput("x".repeat(5000)).length).toBeLessThanOrEqual(2048);
  });
});

describe("slashCommandSkill", () => {
  it("returns a known skill", () => {
    expect(slashCommandSkill("/watchtower new tasks", ["watchtower", "drill-me"])).toBe(
      "watchtower",
    );
  });

  it("returns a plugin-scoped skill", () => {
    expect(slashCommandSkill("/anthropic-skills:pdf", ["anthropic-skills:pdf"])).toBe(
      "anthropic-skills:pdf",
    );
  });

  it("ignores a command that is not a skill", () => {
    expect(slashCommandSkill("/compact", ["watchtower"])).toBeNull();
  });

  it("allows leading whitespace", () => {
    expect(slashCommandSkill("\n  /watchtower", ["watchtower"])).toBe("watchtower");
  });

  it("ignores text that does not start with a slash", () => {
    expect(slashCommandSkill("run /watchtower", ["watchtower"])).toBeNull();
  });
});
