import { settingsRpc, type SettingsDefinition } from "@getpaseo/plugin";
import { useQuery } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { z, type ZodType } from "zod";
import type { BoardHostClient } from "./hosts";

// useSettings follows the surface host. Each lane instead targets its own
// installation, including its revision check when assigning project colors.
export function useHostSettings<Schema extends ZodType>(
  definition: SettingsDefinition<Schema>,
  serverId: string,
  client: BoardHostClient,
  online: boolean,
) {
  const contract = settingsRpc(definition.id);
  const saving = useRef(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const query = useQuery({
    queryKey: ["board-host-settings", serverId, definition.id],
    queryFn: async () => {
      const result = await client.rpc(contract.read, {});
      if (result.status !== "ready") throw new Error(result.error);
      return { revision: result.revision, values: definition.schema.parse(result.values) };
    },
    enabled: online,
    retry: false,
    refetchInterval: 10_000,
    gcTime: 0,
  });
  return {
    status: query.data
      ? ("ready" as const)
      : query.isError
        ? ("error" as const)
        : ("loading" as const),
    values: query.data?.values,
    revision: query.data?.revision,
    saving: saving.current,
    saveError,
    reload: async () => {
      setSaveError(null);
      await query.refetch();
    },
    save: async (values: z.output<Schema>, revision: string | undefined) => {
      if (!online || !revision || saving.current) return false;
      saving.current = true;
      try {
        const result = await client.rpc(contract.write, {
          revision,
          values: z.json().parse(values),
        });
        if (result.status !== "saved") throw new Error(result.error);
        setSaveError(null);
        await query.refetch();
        return true;
      } catch (error) {
        setSaveError(error instanceof Error ? error.message : String(error));
        await query.refetch();
        return false;
      } finally {
        saving.current = false;
      }
    },
  };
}
