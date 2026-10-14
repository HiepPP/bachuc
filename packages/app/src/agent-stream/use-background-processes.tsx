import React, { useCallback, useMemo, type ReactNode } from "react";
import type { AgentBackgroundProcess } from "@getpaseo/protocol/messages";
import type { ToastApi } from "@/components/toast-host";
import { useSessionStore } from "@/stores/session-store";
import type { ToolCallGroupLookup } from "@/tool-calls/detail-level/grouping";
import type { TurnStepGroup } from "@/tool-calls/detail-level/turn/model";
import type { StreamItem } from "@/types/stream";
import {
  BackgroundProcessCard,
  type BackgroundProcessContextValue,
} from "./background-process-card";
import { placeBackgroundProcesses } from "./background-process-placement";

const NO_PROCESSES: AgentBackgroundProcess[] = [];

export function useBackgroundProcesses(input: {
  serverId: string;
  agentId: string;
  tail: readonly StreamItem[];
  head: readonly StreamItem[];
  groupsByHostId: ToolCallGroupLookup<TurnStepGroup>;
  readOnly: boolean;
  toast: ToastApi | null | undefined;
}): { context: BackgroundProcessContextValue; unplacedNode: ReactNode } {
  const { serverId, agentId, tail, head, groupsByHostId, toast } = input;
  const client = useSessionStore((state) => state.sessions[serverId]?.client ?? null);
  const processes = useSessionStore(
    (state) => state.sessions[serverId]?.agents?.get(agentId)?.backgroundProcesses ?? NO_PROCESSES,
  );
  // COMPAT(agentBackgroundProcesses): only a capable daemon can stop a process.
  const supportsStop = useSessionStore(
    (state) => state.sessions[serverId]?.serverInfo?.features?.agentBackgroundProcesses === true,
  );

  const placement = useMemo(
    () => placeBackgroundProcesses({ rows: [...tail, ...head], groupsByHostId, processes }),
    [groupsByHostId, head, processes, tail],
  );
  const stop = useCallback(
    (processId: string) => {
      client?.stopAgentBackgroundProcess(agentId, processId).catch((error: unknown) => {
        toast?.error(error instanceof Error ? error.message : String(error));
      });
    },
    [agentId, client, toast],
  );
  const onStop = supportsStop && !input.readOnly && client ? stop : null;
  const context = useMemo<BackgroundProcessContextValue>(
    () => ({ placement, onStop }),
    [onStop, placement],
  );
  const unplacedNode = useMemo(
    () =>
      placement.unplaced.length > 0 ? (
        <BackgroundProcessCard processes={placement.unplaced} movedCount={0} onStop={onStop} />
      ) : null,
    [onStop, placement.unplaced],
  );
  return { context, unplacedNode };
}
