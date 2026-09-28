import { z } from "zod";

const id = z
  .string()
  .max(64)
  .regex(/^[a-z][a-z0-9-]*$/);
const ids = z.array(id).min(2).max(20);
const nonempty = (limit: number) =>
  z
    .string()
    .max(limit)
    .refine((text) => text.trim().length > 0);
const relations = {
  exclusiveGroups: z.array(ids).max(20),
  allowedCombinations: z.array(ids).max(64),
};

export const selectionSchema = z
  .object({
    id,
    blockKey: z.string(),
    ...relations,
  })
  .strict();
export type Selection = z.infer<typeof selectionSchema>;

const declarationSchema = z
  .object({
    version: z.literal(1),
    prompts: z
      .array(
        z
          .object({
            id,
            prompt: nonempty(16000),
            why: nonempty(2000).optional(),
            // Unrelated to the current goal: offered only as a separate new thread.
            thread: z.literal("new").optional(),
          })
          .strict(),
      )
      .min(1)
      .max(20),
    goal: z.literal("done").optional(),
    exclusiveGroups: relations.exclusiveGroups.default([]),
    allowedCombinations: relations.allowedCombinations.default([]),
  })
  .strict()
  .superRefine((value, ctx) => {
    const known = new Set(value.prompts.map((p) => p.id));
    const separate = new Set(value.prompts.filter((p) => p.thread).map((p) => p.id));
    const grouped = new Set<string>();
    const combinations = new Set<string>();
    const invalid = () =>
      ctx.addIssue({ code: "custom", message: "Invalid prompt relationships." });
    if (known.size !== value.prompts.length) invalid();
    for (const list of [...value.exclusiveGroups, ...value.allowedCombinations])
      if (
        new Set(list).size !== list.length ||
        list.some((key) => !known.has(key) || separate.has(key))
      )
        invalid();
    // Disjoint groups map directly to native radio groups, including keyboard navigation.
    for (const group of value.exclusiveGroups)
      for (const key of group) {
        if (grouped.has(key)) invalid();
        grouped.add(key);
      }
    for (const combination of value.allowedCombinations) {
      const key = [...combination].sort().join("\n");
      if (combinations.has(key)) invalid();
      combinations.add(key);
      if (
        value.exclusiveGroups.some(
          (group) => group.filter((item) => combination.includes(item)).length > 1,
        )
      )
        invalid();
    }
  });
export type NextPromptsV1 = z.infer<typeof declarationSchema>;

export function parseNextPrompts(block: string): NextPromptsV1 | null {
  if (block.length > 65536 || new TextEncoder().encode(block).byteLength > 65536) return null;
  try {
    const parsed = declarationSchema.safeParse(JSON.parse(block));
    return parsed.success ? parsed.data : null;
  } catch {
    return null;
  }
}

export function selectionAllowed(
  candidates: readonly { key: string; selection?: Selection }[],
): boolean {
  if (!candidates.length || new Set(candidates.map((c) => c.key)).size !== candidates.length)
    return false;
  if (candidates.length === 1) return true;
  const first = candidates[0].selection;
  if (!first || candidates.some((c) => !c.selection || c.selection.blockKey !== first.blockKey))
    return false;
  const selected = new Set(candidates.map((c) => c.selection!.id));
  if (selected.size !== candidates.length) return false;
  if (first.exclusiveGroups.some((group) => group.filter((id) => selected.has(id)).length > 1))
    return false;
  return first.allowedCombinations.some(
    (group) => group.length === selected.size && group.every((id) => selected.has(id)),
  );
}
