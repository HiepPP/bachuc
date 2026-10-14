import { useEffect, useMemo } from "react";
import type { InstalledPlugin } from "./types";
import { createPaseoApi, type PaseoApi } from "@getpaseo/client";
import type { DaemonClient } from "@getpaseo/client/internal/daemon-client";

export interface PluginSurfaceRuntime {
  paseo: PaseoApi;
  invoke(method: string, input: unknown): Promise<unknown>;
}

export function createPluginSurfaceRuntime(
  client: DaemonClient | null,
  plugin: Pick<InstalledPlugin, "id" | "lifetime">,
): PluginSurfaceRuntime | null {
  if (!client || plugin.lifetime.signal.aborted) return null;
  return {
    paseo: createPaseoApi(client, { signal: plugin.lifetime.signal }),
    invoke: (method, input) => client.invokePluginRpc(plugin.id, method, input),
  };
}

const mountedRuntimeCounts = new WeakMap<PluginSurfaceRuntime, number>();

/**
 * A mounted surface owns its API; creating a React element creates no server demand. The runtime
 * exists from the first render, so plugin content commits with its host row instead of a frame
 * later, which a virtualized timeline would measure as an empty row. An API from a render React
 * discards holds no server demand until used and is released with the plugin lifetime.
 */
export function usePluginSurfaceRuntime(
  client: DaemonClient | null,
  plugin: InstalledPlugin | null | undefined,
): PluginSurfaceRuntime | null {
  const runtime = useMemo(
    () => (client && plugin ? createPluginSurfaceRuntime(client, plugin) : null),
    [client, plugin],
  );
  const pluginId = plugin?.id;
  useEffect(() => {
    if (!runtime) return;
    mountedRuntimeCounts.set(runtime, (mountedRuntimeCounts.get(runtime) ?? 0) + 1);
    return () => {
      mountedRuntimeCounts.set(runtime, (mountedRuntimeCounts.get(runtime) ?? 1) - 1);
      // StrictMode replays effects within the same task, so dispose only once no effect took the
      // runtime back.
      queueMicrotask(() => {
        if ((mountedRuntimeCounts.get(runtime) ?? 0) > 0) return;
        void runtime.paseo
          .dispose()
          .catch((error) =>
            console.warn(`[Plugins] Surface cleanup failed for ${pluginId}`, error),
          );
      });
    };
  }, [pluginId, runtime]);
  return runtime;
}
