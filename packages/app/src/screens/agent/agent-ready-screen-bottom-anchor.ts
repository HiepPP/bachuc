import type { BottomAnchorRouteRequest } from "@/agent-stream/bottom-anchor-controller";

export interface RouteBottomAnchorIntent {
  routeKey: string;
  /** Null when the route opened on a latest-prompt anchor and must not pin to the bottom. */
  reason: BottomAnchorRouteRequest["reason"] | null;
}

export function deriveRouteBottomAnchorIntent(input: {
  cachedIntent: RouteBottomAnchorIntent | null;
  routeKey: string | null;
  hasAppliedAuthoritativeHistoryAtEntry: boolean;
  hasPendingLatestPromptAnchorAtEntry: boolean;
}): RouteBottomAnchorIntent | null {
  if (!input.routeKey) {
    return null;
  }
  if (input.cachedIntent?.routeKey === input.routeKey) {
    return input.cachedIntent;
  }
  if (input.hasPendingLatestPromptAnchorAtEntry) {
    return { routeKey: input.routeKey, reason: null };
  }
  return {
    routeKey: input.routeKey,
    reason: input.hasAppliedAuthoritativeHistoryAtEntry ? "resume" : "initial-entry",
  };
}

export function deriveRouteBottomAnchorRequest(input: {
  intent: RouteBottomAnchorIntent | null;
  effectiveAgentId: string | null;
}): BottomAnchorRouteRequest | null {
  if (!input.intent?.reason || !input.effectiveAgentId) {
    return null;
  }
  return {
    reason: input.intent.reason,
    agentId: input.effectiveAgentId,
    requestKey: `${input.intent.routeKey}:${input.intent.reason}`,
  };
}
