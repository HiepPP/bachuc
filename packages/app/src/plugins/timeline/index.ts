import { useMemo } from "react";
import { useSessionStore } from "@/stores/session-store";
import { useInstalledPlugins } from "../registry";
import { transformTimelineItem, type TimelineItemTransform } from "./model";

export type { InstalledPluginTimelineItem, TimelineItemTransform } from "./model";
export { PluginTimelineItemView } from "./view";

export function useInstalledTimelineTransform(
  serverId: string,
  agentId: string,
): TimelineItemTransform {
  const installed = useInstalledPlugins();
  const plugins = useMemo(
    () => installed.filter((plugin) => plugin.serverId === serverId),
    [installed, serverId],
  );
  // A new transform drops the projection cache, so key it on the label values, not the object
  // that every agent update replaces.
  const labelsKey = useSessionStore((state) =>
    JSON.stringify(state.sessions[serverId]?.agents?.get(agentId)?.labels ?? {}),
  );
  return useMemo(() => {
    const labels = Object.freeze(JSON.parse(labelsKey) as Record<string, string>);
    const agent = Object.freeze({ id: agentId, labels });
    return (input) => transformTimelineItem({ ...input, plugins, agent });
  }, [agentId, labelsKey, plugins]);
}
