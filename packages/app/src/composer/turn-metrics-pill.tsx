import { memo, useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { View } from "react-native";
import { useShallow } from "zustand/shallow";
import { TurnMetricsText } from "@/components/turn-metrics-text";
import { useComposerPillStyles } from "@/composer/pill-styles";
import { selectAgentTurnPresentation, useSessionStore } from "@/stores/session-store";
import {
  buildComposerTurnMetricParts,
  deriveComposerTurnMetrics,
  findLastTurnMetrics,
} from "@/timeline/turn-time";
import type { StreamItem } from "@/types/stream";

const EMPTY_ITEMS: StreamItem[] = [];
const LIVE_TICK_MS = 1000;

/**
 * Whether the composer has turn metrics to show. A boolean selector, so the composer strip
 * does not re-render on every streamed chunk or live tick.
 */
export function useHasComposerTurnMetrics(serverId: string, agentId: string): boolean {
  return useSessionStore((state) => {
    const session = state.sessions[serverId];
    if (selectAgentTurnPresentation(session, agentId).isActive) return true;
    return (
      findLastTurnMetrics([
        session?.agentStreamTail?.get(agentId),
        session?.agentStreamHead?.get(agentId),
      ]) !== null
    );
  });
}

/** TTFT and output rate of the agent's running turn, or of its last turn once it ends. */
export const ComposerTurnMetricsPill = memo(function ComposerTurnMetricsPill({
  serverId,
  agentId,
}: {
  serverId: string;
  agentId: string;
}): ReactElement | null {
  const composerPillStyles = useComposerPillStyles();
  const tail = useSessionStore(
    (state) => state.sessions[serverId]?.agentStreamTail?.get(agentId) ?? EMPTY_ITEMS,
  );
  const head = useSessionStore(
    (state) => state.sessions[serverId]?.agentStreamHead?.get(agentId) ?? EMPTY_ITEMS,
  );
  const turn = useSessionStore(
    useShallow((state) => selectAgentTurnPresentation(state.sessions[serverId], agentId)),
  );
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!turn.isActive) return;
    setNow(Date.now());
    const handle = setInterval(() => setNow(Date.now()), LIVE_TICK_MS);
    return () => clearInterval(handle);
  }, [turn.isActive]);

  // Earliest first-output time seen per turn; see deriveComposerTurnMetrics.
  const seenFirstOutputRef = useRef<{ turnKey: string; at: number } | null>(null);
  const turnKey = `${turn.turnId ?? ""}:${turn.startedAt?.getTime() ?? ""}`;
  const metrics = useMemo(() => {
    const seen = seenFirstOutputRef.current;
    const result = deriveComposerTurnMetrics({
      items: head.length > 0 ? [...tail, ...head] : tail,
      isTurnActive: turn.isActive,
      turnStartedAt: turn.startedAt,
      turnId: turn.turnId,
      now,
      seenFirstOutputAt: seen?.turnKey === turnKey ? seen.at : null,
    });
    if (result?.kind === "streaming") {
      seenFirstOutputRef.current = { turnKey, at: result.firstOutputAt };
    }
    return result;
  }, [head, now, tail, turn.isActive, turn.startedAt, turn.turnId, turnKey]);
  const parts = useMemo(() => buildComposerTurnMetricParts(metrics), [metrics]);
  if (parts.length === 0) {
    return null;
  }
  return (
    <View style={composerPillStyles.body} testID="composer-turn-metrics-pill">
      <TurnMetricsText parts={parts} style={composerPillStyles.label} />
    </View>
  );
});
