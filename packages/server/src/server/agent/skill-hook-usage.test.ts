import { describe, expect, it } from "vitest";
import { applySkillHookEvent } from "./skill-hook-usage.js";

const T1 = "2026-10-27T10:00:00.000Z";
const T2 = "2026-10-27T10:01:00.000Z";

describe("applySkillHookEvent", () => {
  it("starts a summary from nothing", () => {
    expect(applySkillHookEvent(undefined, { kind: "skill", name: "watchtower" }, T1)).toEqual({
      skills: [{ name: "watchtower", count: 1, errorCount: 0, lastUsedAt: T1 }],
      hooks: [],
    });
  });

  it("counts a skill again and moves it to the front", () => {
    let usage = applySkillHookEvent(undefined, { kind: "skill", name: "a" }, T1);
    usage = applySkillHookEvent(usage, { kind: "skill", name: "b" }, T1);
    usage = applySkillHookEvent(usage, { kind: "skill", name: "a" }, T2);
    expect(usage.skills.map((entry) => [entry.name, entry.count])).toEqual([
      ["a", 2],
      ["b", 1],
    ]);
    expect(usage.skills[0]?.lastUsedAt).toBe(T2);
  });

  it("counts hook errors", () => {
    let usage = applySkillHookEvent(
      undefined,
      { kind: "hook", name: "PreToolUse:Bash", event: "PreToolUse", status: "success" },
      T1,
    );
    usage = applySkillHookEvent(
      usage,
      { kind: "hook", name: "PreToolUse:Bash", event: "PreToolUse", status: "error" },
      T2,
    );
    expect(usage.hooks).toEqual([
      { name: "PreToolUse:Bash", event: "PreToolUse", count: 2, errorCount: 1, lastUsedAt: T2 },
    ]);
  });

  it("keeps the 50 newest entries", () => {
    let usage = applySkillHookEvent(undefined, { kind: "skill", name: "s0" }, T1);
    for (let index = 1; index <= 55; index += 1) {
      usage = applySkillHookEvent(usage, { kind: "skill", name: `s${index}` }, T1);
    }
    expect(usage.skills).toHaveLength(50);
    expect(usage.skills[0]?.name).toBe("s55");
    expect(usage.skills.some((entry) => entry.name === "s0")).toBe(false);
  });

  it("does not change the input summary", () => {
    const before = applySkillHookEvent(undefined, { kind: "skill", name: "a" }, T1);
    applySkillHookEvent(before, { kind: "skill", name: "a" }, T2);
    expect(before.skills[0]?.count).toBe(1);
  });
});
