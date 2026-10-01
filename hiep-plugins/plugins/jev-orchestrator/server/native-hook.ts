import { z } from "zod";
import { effortSchema, isLunaModel } from "../shared/direct";
import type { Judge, Profile } from "./types";

const nonBlank = (maximum: number) =>
  z
    .string()
    .min(1)
    .max(maximum)
    .refine((value) => value.trim().length > 0, "Value cannot be blank.");

const nativeCandidateSchema = z.strictObject({
  agentType: nonBlank(100),
  model: nonBlank(150),
  effort: effortSchema,
  description: nonBlank(1000),
});

const nativeRouteSchema = z.strictObject({
  sourceType: nonBlank(100),
  candidates: z.array(nativeCandidateSchema).min(1).max(8),
});

export const nativePolicySchema = z
  .strictObject({
    runtime: z.enum(["codex", "claude"]),
    routes: z.array(nativeRouteSchema).min(1).max(50),
  })
  .superRefine((policy, context) => {
    const sourceTypes = new Set<string>();
    const generatedTypes = new Set<string>();

    for (const [routeIndex, route] of policy.routes.entries()) {
      if (sourceTypes.has(route.sourceType)) {
        context.addIssue({
          code: "custom",
          path: ["routes", routeIndex, "sourceType"],
          message: "Duplicate source type.",
        });
      }
      sourceTypes.add(route.sourceType);

      const pairs = new Set<string>();
      for (const [candidateIndex, candidate] of route.candidates.entries()) {
        if (generatedTypes.has(candidate.agentType)) {
          context.addIssue({
            code: "custom",
            path: ["routes", routeIndex, "candidates", candidateIndex, "agentType"],
            message: "Duplicate generated agent type.",
          });
        }
        generatedTypes.add(candidate.agentType);

        const pair = JSON.stringify([candidate.model, candidate.effort]);
        if (pairs.has(pair)) {
          context.addIssue({
            code: "custom",
            path: ["routes", routeIndex, "candidates", candidateIndex],
            message: "Duplicate model and effort pair.",
          });
        }
        pairs.add(pair);

        if (isLunaModel(candidate.model) && candidate.effort !== "max") {
          context.addIssue({
            code: "custom",
            path: ["routes", routeIndex, "candidates", candidateIndex, "effort"],
            message: "Luna only permits max effort.",
          });
        }
      }
    }

    for (const generatedType of generatedTypes) {
      if (sourceTypes.has(generatedType)) {
        context.addIssue({
          code: "custom",
          path: ["routes"],
          message: "Generated agent types cannot be routed source types.",
        });
      }
    }
  });

export type NativePolicy = z.infer<typeof nativePolicySchema>;
export type NativeRoute = NativePolicy["routes"][number];
export type NativeCandidate = NativeRoute["candidates"][number];
export type NativeHookEvent = Record<string, unknown> & {
  hook_event_name?: unknown;
  tool_name?: unknown;
  tool_input?: unknown;
};
// An empty object leaves the tool call to the provider's normal permission flow.
export interface NativeHookOutput {
  hookSpecificOutput?: {
    hookEventName: "PreToolUse";
    permissionDecision?: "allow" | "deny";
    permissionDecisionReason?: string;
    updatedInput?: Record<string, unknown>;
  };
}

const TIMED_OUT = "Jev native routing timed out. No subagent was started.";

const denied = (reason: string): NativeHookOutput => ({
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: reason,
  },
});

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

interface NativeRequest {
  task: string;
  role: string;
  input: Record<string, unknown>;
}

// Returns a deny reason when the event is not a routable subagent start.
function taskAndRole(
  event: NativeHookEvent,
  runtime: NativePolicy["runtime"],
): NativeRequest | string {
  if (event.hook_event_name !== "PreToolUse" || !isRecord(event.tool_input))
    return "Native subagent routing denied: unsupported hook event.";
  const input = event.tool_input;
  if (Object.hasOwn(input, "resume") && input.resume != null)
    return "Native subagent routing denied: resuming a subagent is not routed. Start a new subagent.";

  if (runtime === "codex") {
    if (event.tool_name !== "spawn_agent" && event.tool_name !== "Agent")
      return "Native subagent routing denied: unsupported tool.";
    if (typeof input.message !== "string" || input.message.trim().length === 0)
      return "Native subagent routing denied: the subagent message is empty.";
    if (input.agent_type !== undefined && typeof input.agent_type !== "string")
      return "Native subagent routing denied: agent_type must be a string.";
    return { task: input.message, role: input.agent_type ?? "default", input };
  }

  if (event.tool_name !== "Agent" && event.tool_name !== "Task")
    return "Native subagent routing denied: unsupported tool.";
  if (typeof input.prompt !== "string" || input.prompt.trim().length === 0)
    return "Native subagent routing denied: the subagent prompt is empty.";
  // Claude Code starts general-purpose when subagent_type is omitted.
  if (input.subagent_type === undefined)
    return { task: input.prompt, role: "general-purpose", input };
  if (typeof input.subagent_type !== "string" || input.subagent_type.trim().length === 0)
    return "Native subagent routing denied: subagent_type must be a non-empty string.";
  return { task: input.prompt, role: input.subagent_type, input };
}

export async function routeNativeHook(
  eventInput: unknown,
  policyInput: unknown,
  judge: Judge,
  signal?: AbortSignal,
): Promise<NativeHookOutput> {
  const controller = signal ? undefined : new AbortController();
  const timer = controller ? setTimeout(() => controller.abort(), 25_000) : undefined;
  timer?.unref();
  const activeSignal = signal ?? controller!.signal;
  try {
    const parsed = nativePolicySchema.safeParse(policyInput);
    if (!parsed.success)
      return denied(
        "Jev native routing unavailable: invalid routing policy. Create a fresh Paseo agent.",
      );
    if (!isRecord(eventInput))
      return denied("Native subagent routing denied: malformed hook event.");
    if (activeSignal.aborted) return denied(TIMED_OUT);

    const policy = parsed.data;
    // Codex 0.154 concatenates the namespace and tool name. Its hook event lacks
    // the plaintext/encrypted message discriminator; do not evaluate opaque tasks.
    if (policy.runtime === "codex" && eventInput.tool_name === "collaborationspawn_agent")
      return denied(
        "Jev cannot route native v2: Codex does not expose a verifiable plaintext task to this hook. No subagent was started. Do not retry or bypass routing.",
      );
    const request = taskAndRole(eventInput, policy.runtime);
    if (typeof request === "string") return denied(request);
    const route = policy.routes.find(({ sourceType }) => sourceType === request.role);
    if (!route) {
      const reserved =
        request.role.startsWith("jev-native-") ||
        policy.routes.some(({ candidates }) =>
          candidates.some(({ agentType }) => agentType === request.role),
        );
      // Unrouted roles keep the provider's own definition; reserved types would skip Jev's choice.
      return reserved
        ? denied(
            `Native subagent routing denied: ${request.role} is reserved for Jev routing. Request its source agent type instead.`,
          )
        : {};
    }

    const profiles: Profile[] = route.candidates.map((candidate, index) => ({
      id: `c${index}`,
      name: candidate.agentType,
      provider: policy.runtime,
      model: candidate.model,
      thinkingOptionId: candidate.effort,
      notes: candidate.description,
    }));
    const allowedPairs = route.candidates.map((candidate, index) => ({
      id: `c${index}`,
      agentType: candidate.agentType,
      model: candidate.model,
      effort: candidate.effort,
    }));

    try {
      const decision = await judge(
        "direct",
        { task: request.task, originalRole: request.role, allowedPairs },
        profiles,
        activeSignal,
      );
      if (activeSignal.aborted) return denied(TIMED_OUT);
      const selectedIndex = profiles.findIndex(({ id }) => id === decision.profileId);
      if (selectedIndex < 0)
        return denied(
          "Jev native routing denied: Jev chose a profile outside this route. No subagent was started.",
        );
      const selected = route.candidates[selectedIndex];

      if (policy.runtime === "codex") {
        const updatedInput: Record<string, unknown> = {
          ...request.input,
          agent_type: selected.agentType,
        };
        delete updatedInput.model;
        delete updatedInput.reasoning_effort;
        delete updatedInput.model_reasoning_effort;
        delete updatedInput.effort;
        return {
          hookSpecificOutput: {
            hookEventName: "PreToolUse",
            permissionDecision: "allow",
            updatedInput,
          },
        };
      }

      // Agent's model field accepts only aliases; the pinned definition carries the full model ID.
      const updatedInput: Record<string, unknown> = {
        ...request.input,
        subagent_type: selected.agentType,
      };
      delete updatedInput.model;
      return { hookSpecificOutput: { hookEventName: "PreToolUse", updatedInput } };
    } catch {
      // Judge errors can echo task text; keep them out of the transcript.
      return denied(
        activeSignal.aborted
          ? TIMED_OUT
          : "Jev native routing unavailable: Jev evaluation failed. No subagent was started. Check jev-orchestrator logs.",
      );
    }
  } finally {
    if (timer) clearTimeout(timer);
  }
}
