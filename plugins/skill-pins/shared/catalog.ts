import { z } from "zod";

// `claude` is the Skill tool name. Every entry lives in ~/.claude/skills/<id>/SKILL.md.
// `reinvoke` marks a skill that performs work instead of setting a method. Loading it once is
// not the same as running it, so the hook asks for it again on every turn.
export const catalog = [
  {
    id: "watchtower",
    label: "Watchtower",
    short: "Watch",
    description: "Plan and track watchtower tasks",
    claude: "watchtower",
  },
  {
    id: "chase-goal-claude",
    label: "Chase goal",
    short: "Goal",
    description: "Plan and run a goal with subagents",
    warn: "Starts goal work on every turn",
    claude: "chase-goal-claude",
    reinvoke: true,
  },
  {
    id: "sequential-thinking",
    label: "Sequential thinking",
    short: "Seq",
    description: "Step-by-step reasoning with revisions",
    claude: "sequential-thinking",
  },
] as const;

export type SkillId = (typeof catalog)[number]["id"];

const ids = catalog.map((skill) => skill.id) as [SkillId, ...SkillId[]];
export const skillIdSchema = z.enum(ids);
// Stored and sent in catalog order without duplicates, so equal selections compare equal.
export const skillsSchema = z
  .array(skillIdSchema)
  .transform((skills) => ids.filter((id) => skills.includes(id)));
// Saved files may hold IDs from an older catalog; reading drops them instead of failing.
export const savedSkillsSchema = z
  .array(z.string())
  .catch([])
  .transform((skills) => ids.filter((id) => skills.includes(id)));
