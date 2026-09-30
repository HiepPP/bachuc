import type { PluginClientContext, PluginSurfaceProps } from "@getpaseo/plugin/client";
import { useEffect, useState, useSyncExternalStore } from "react";
import { Text } from "react-native";
import type { BoardRun } from "../shared/board";
import { openNewWorkspaceForProject } from "./web";

// Composer actions lack the host id the New workspace route needs; surfaces receive it.
export function installNewThreadNavigation(client: PluginClientContext) {
  let target: BoardRun | null = null;
  const listeners = new Set<() => void>();
  const subscribe = (listener: () => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };
  const snapshot = () => target;
  function NewThread({ host, theme }: PluginSurfaceProps) {
    const run = useSyncExternalStore(subscribe, snapshot);
    const [failed, setFailed] = useState(false);
    useEffect(() => {
      if (!run?.cwd) return;
      setFailed(
        !openNewWorkspaceForProject({
          serverId: host.id,
          cwd: run.cwd,
          name: run.project,
          projectId: run.projectId,
        }),
      );
    }, [run, host.id]);
    return (
      <Text style={{ color: theme.colors.foregroundMuted }}>
        {failed ? "Starting a new thread needs the desktop app." : "Opening project…"}
      </Text>
    );
  }
  const cleanup = client.addSurface("new-thread", NewThread);
  return {
    open(run: BoardRun) {
      target = { ...run };
      client.openSurface("new-thread");
      for (const listener of listeners) listener();
    },
    cleanup,
  };
}
