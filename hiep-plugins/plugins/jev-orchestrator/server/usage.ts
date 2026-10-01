/** Input includes cache reads/writes; output includes reasoning where the provider reports it. */
export interface TokenUsage {
  source: string;
  complete: boolean;
  inputTokens: number | null;
  cachedInputTokens: number | null;
  cacheWriteTokens: number | null;
  outputTokens: number | null;
  reasoningTokens: number | null;
  totalTokens: number | null;
  requests: number | null;
  notes: string[];
}
export function missingUsage(source: string, note: string): TokenUsage {
  return {
    source,
    complete: false,
    inputTokens: null,
    cachedInputTokens: null,
    cacheWriteTokens: null,
    outputTokens: null,
    reasoningTokens: null,
    totalTokens: null,
    requests: null,
    notes: [note],
  };
}
export const tokenCount = (value: unknown): number | null =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : null;
export function evaluationUsage(raw: {
  inputTokens?: number | null;
  outputTokens?: number | null;
}): TokenUsage {
  const inputTokens = tokenCount(raw.inputTokens),
    outputTokens = tokenCount(raw.outputTokens);
  const complete = inputTokens !== null && outputTokens !== null;
  return {
    source: "typesafe-evaluate-response",
    complete,
    inputTokens,
    outputTokens,
    totalTokens: complete ? inputTokens! + outputTokens! : null,
    cachedInputTokens: null,
    cacheWriteTokens: null,
    reasoningTokens: null,
    requests: 1,
    notes: complete
      ? ["Typesafe evaluation API does not expose cache/reasoning breakdown."]
      : ["Typesafe evaluation response omitted token counts; do not substitute zero."],
  };
}
export function summarizeUsage(components: { id: string; role: string; usage: TokenUsage }[]) {
  if (new Set(components.map((c) => c.id)).size !== components.length)
    throw new Error("Duplicate accounting component.");
  const fields = [
    "inputTokens",
    "cachedInputTokens",
    "cacheWriteTokens",
    "outputTokens",
    "reasoningTokens",
    "totalTokens",
  ] as const;
  const complete =
    components.length > 0 &&
    components.every((c) => c.usage.complete && c.usage.totalTokens !== null);
  const totals = Object.fromEntries(
    fields.map((field) => [
      field,
      components.length &&
      (field !== "totalTokens" || complete) &&
      components.every((c) => c.usage[field] !== null)
        ? components.reduce((sum, c) => sum + c.usage[field]!, 0)
        : null,
    ]),
  );
  // A partial sum is reported separately and never labeled a complete workflow total.
  return {
    complete,
    totals,
    knownTotalTokens: components.reduce((sum, c) => sum + (c.usage.totalTokens ?? 0), 0),
    missing: components
      .filter((c) => !c.usage.complete || c.usage.totalTokens === null)
      .map((c) => c.id),
    components,
  };
}
