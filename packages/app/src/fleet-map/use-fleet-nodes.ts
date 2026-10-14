import equal from "fast-deep-equal";
import { useStoreWithEqualityFn } from "zustand/traditional";
import { useSessionStore } from "@/stores/session-store";
import { selectFleetNodes, type FleetNode } from "./nodes";

export function useFleetNodes(serverId: string): FleetNode[] {
  return useStoreWithEqualityFn(
    useSessionStore,
    (state) => selectFleetNodes(state.sessions[serverId]),
    equal,
  );
}
