import { useEffect, useRef, useSyncExternalStore } from "react";
import { isWeb } from "@/constants/platform";
import { getHostRuntimeStore } from "@/runtime/host-runtime";
import { selectAgentTimelineState, useSessionStore } from "@/stores/session-store";
import { planTimelinePromptJump } from "@/timeline/timeline-sync-plan";
import type { StreamItem } from "@/types/stream";
import { shouldAcceptPromptIndexEpoch } from "./chat-outline/model";
import {
  findLatestPromptId,
  getLatestPromptAnchorVersion,
  hasPendingLatestPromptAnchor,
  settleLatestPromptAnchor,
  subscribeLatestPromptAnchor,
} from "./latest-prompt-anchor";

interface UseLatestPromptAnchorLoaderInput {
  agentId: string;
  serverId: string;
  timelineEpoch: string | null;
  tail: StreamItem[];
  head: StreamItem[] | undefined;
  /** The host serves the prompt index. */
  enabled: boolean;
  isActive: boolean;
  visibleMessageIds: ReadonlySet<string>;
  revealLoadedMessage: (messageId: string) => boolean;
}

const NO_STREAM_ITEMS: StreamItem[] = [];

/**
 * Brings the latest prompt into the mounted transcript for a pending latest-prompt anchor. The
 * web viewport scrolls to it once it is mounted; this only loads and reveals it.
 */
export function useLatestPromptAnchorLoader({
  agentId,
  serverId,
  timelineEpoch,
  tail,
  head,
  enabled,
  isActive,
  visibleMessageIds,
  revealLoadedMessage,
}: UseLatestPromptAnchorLoaderInput): void {
  const version = useSyncExternalStore(
    subscribeLatestPromptAnchor,
    getLatestPromptAnchorVersion,
    getLatestPromptAnchorVersion,
  );
  const isTimelineSynced = useSessionStore(
    (state) => selectAgentTimelineState(state.sessions[serverId], agentId).status === "synced",
  );
  const fetchedVersionRef = useRef<number | null>(null);

  useEffect(() => {
    if (!isWeb || !isActive || !hasPendingLatestPromptAnchor(agentId)) return;
    const loadedId = findLatestPromptId([tail, head ?? NO_STREAM_ITEMS]);
    if (loadedId) {
      if (!visibleMessageIds.has(loadedId)) revealLoadedMessage(loadedId);
      return;
    }
    // A synced timeline with no loaded prompt and no prompt index has nothing to land on.
    if (!enabled) {
      if (isTimelineSynced) settleLatestPromptAnchor(agentId);
      return;
    }
    // One fetch per request: a timeline without prompts must not loop.
    if (fetchedVersionRef.current === version) return;
    const client = getHostRuntimeStore().getClient(serverId);
    if (!client) return;
    fetchedVersionRef.current = version;
    void client
      .listAgentTimelinePrompts(agentId)
      .then((index) => {
        const latest = index.prompts.at(-1);
        if (!latest || !shouldAcceptPromptIndexEpoch(timelineEpoch, index.epoch)) {
          settleLatestPromptAnchor(agentId);
          return undefined;
        }
        return getHostRuntimeStore().fetchAgentTimeline(
          serverId,
          agentId,
          planTimelinePromptJump({ epoch: index.epoch, seq: latest.seq }),
        );
      })
      .catch((error: unknown) => {
        console.warn("Failed to load the latest prompt for an anchor", error);
        settleLatestPromptAnchor(agentId);
      });
  }, [
    agentId,
    enabled,
    head,
    isActive,
    isTimelineSynced,
    revealLoadedMessage,
    serverId,
    tail,
    timelineEpoch,
    version,
    visibleMessageIds,
  ]);
}
