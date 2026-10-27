import type { AgentSkillHookUsage, SkillHookUsageEntry, ToolCallHook } from "./agent-sdk-types.js";

export const MAX_SKILL_HOOK_USAGE_ENTRIES = 50;

export type SkillHookEvent =
  | { kind: "skill"; name: string }
  | { kind: "hook"; name: string; event: string; status: ToolCallHook["status"] };

/** Counts one skill use or one finished hook. Returns a new summary; the input is not changed. */
export function applySkillHookEvent(
  usage: AgentSkillHookUsage | undefined,
  event: SkillHookEvent,
  at: string,
): AgentSkillHookUsage {
  const current = usage ?? { skills: [], hooks: [] };
  if (event.kind === "skill") {
    return { ...current, skills: bump(current.skills, { name: event.name }, false, at) };
  }
  // A hook that still runs is not a use yet.
  if (event.status === "running") return current;
  return {
    ...current,
    hooks: bump(
      current.hooks,
      { name: event.name, event: event.event },
      event.status === "error",
      at,
    ),
  };
}

function bump(
  entries: SkillHookUsageEntry[],
  key: { name: string; event?: string },
  isError: boolean,
  at: string,
): SkillHookUsageEntry[] {
  const previous = entries.find((entry) => entry.name === key.name);
  const next: SkillHookUsageEntry = {
    name: key.name,
    ...(key.event === undefined ? {} : { event: key.event }),
    count: (previous?.count ?? 0) + 1,
    errorCount: (previous?.errorCount ?? 0) + (isError ? 1 : 0),
    lastUsedAt: at,
  };
  return [next, ...entries.filter((entry) => entry.name !== key.name)].slice(
    0,
    MAX_SKILL_HOOK_USAGE_ENTRIES,
  );
}

export function hasSkillHookUsage(usage: AgentSkillHookUsage | undefined): boolean {
  return usage !== undefined && (usage.skills.length > 0 || usage.hooks.length > 0);
}
