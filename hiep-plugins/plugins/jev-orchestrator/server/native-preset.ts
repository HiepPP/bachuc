import { nativePolicySchema, type NativePolicy } from "./native-hook";

// User-selected pairs, verified against the installed provider catalog.
// These are routing preferences, not measured cost or quality guarantees.
const pairs = {
  codex: [
    {
      model: "gpt-6-astra",
      effort: "low",
      suffix: "astra-low",
      description:
        "User policy: choose for difficult or ambiguous work, unresolved diagnosis, design decisions or interacting constraints, even if the parent supplied a spec.",
    },
    {
      model: "gpt-5.6-luna",
      effort: "max",
      suffix: "luna-max",
      description:
        "User policy: preferred for parent-delegated execution with a clear spec, bounded scope and explicit acceptance criteria. Do not choose merely because the prompt contains the word spec; choose the difficult-work candidate if material ambiguity remains. Cost savings are a target, not a measured guarantee.",
    },
  ],
  claude: [
    {
      model: "claude-opus-5-5",
      effort: "medium",
      suffix: "opus-medium",
      description:
        "User policy: preferred for parent-delegated execution with a clear spec, bounded scope and explicit acceptance criteria, such as routine implementation. Do not choose merely because the prompt contains the word spec; choose a higher effort if material ambiguity remains.",
    },
    {
      model: "claude-opus-5-5",
      effort: "high",
      suffix: "opus-high",
      description:
        "User policy: choose where verification matters or edge cases are likely, such as fixing a bug in existing code, reviews, or changes across several files.",
    },
    {
      model: "claude-opus-5-5",
      effort: "xhigh",
      suffix: "opus-xhigh",
      description:
        "User policy: choose for difficult or ambiguous work, unresolved diagnosis, design decisions, security-sensitive changes or interacting constraints, even if the parent supplied a spec.",
    },
  ],
} as const;

export function nativePreset(
  runtime: NativePolicy["runtime"],
  sourceTypes: string[],
): NativePolicy {
  return nativePolicySchema.parse({
    runtime,
    routes: sourceTypes.map((sourceType, index) => ({
      sourceType,
      candidates: pairs[runtime].map(({ suffix, ...candidate }) =>
        Object.assign(candidate, { agentType: `jev-${runtime}-r${index}-${suffix}` }),
      ),
    })),
  });
}
