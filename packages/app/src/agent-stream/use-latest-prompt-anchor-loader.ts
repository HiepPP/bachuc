import { useEffect, useRef, useSyncExternalStore } from "react";
import { isWeb } from "@/constants/platform";
import { getHostRuntimeStore } from "@/runtime/host-runtime";
import { planTimelinePromptJump } from "@/timeline/timeline-sync-plan";
import type { StreamItem } from "@/types/stream";
import { shouldAcceptPromptIndexEpoch } from "./chat-outline/model";
import {
  extendLatestPromptAnchor,
  findLatestPromptId,
  getLatestPromptAnchorVersion,
  hasPendingLatestPromptAnchor,
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
  const fetchedVersionRef = useRef<number | null>(null);

  useEffect(() => {
    if (!isWeb || !isActive || !hasPendingLatestPromptAnchor(agentId)) return;
    const loadedId = findLatestPromptId([tail, head ?? NO_STREAM_ITEMS]);
    if (loadedId) {
      if (!visibleMessageIds.has(loadedId)) revealLoadedMessage(loadedId);
      return;
    }
    // One fetch per request: a timeline without prompts must not loop.
    if (!enabled || fetchedVersionRef.current === version) return;
    const client = getHostRuntimeStore().getClient(serverId);
    if (!client) return;
    fetchedVersionRef.current = version;
    void client
      .listAgentTimelinePrompts(agentId)
      .then((index) => {
        const latest = index.prompts.at(-1);
        if (!latest || !shouldAcceptPromptIndexEpoch(timelineEpoch, index.epoch)) return undefined;
        return getHostRuntimeStore().fetchAgentTimeline(
          serverId,
          agentId,
          planTimelinePromptJump({ epoch: index.epoch, seq: latest.seq }),
        );
      })
      .then(() => extendLatestPromptAnchor(agentId))
      .catch((error: unknown) => {
        console.warn("Failed to load the latest prompt for an anchor", error);
      });
  }, [
    agentId,
    enabled,
    head,
    isActive,
    revealLoadedMessage,
    serverId,
    tail,
    timelineEpoch,
    version,
    visibleMessageIds,
  ]);
}
