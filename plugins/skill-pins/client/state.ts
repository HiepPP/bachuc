import type { PluginClientContext } from "@getpaseo/plugin/client";
import { draftReadRpc, draftWriteRpc, skillsReadRpc, skillsWriteRpc } from "../shared/contracts";
import type { SkillId } from "../shared/catalog";

// Key for new-thread composers, which have no agent ID yet.
export const DRAFT = "draft";

export function createSkillState(client: Pick<PluginClientContext, "rpc">) {
  const values = new Map<string, SkillId[]>();
  const loading = new Map<string, Promise<void>>();
  const listeners = new Set<() => void>();
  const notify = () => listeners.forEach((fn) => fn());
  return {
    get(agentId: string): SkillId[] | undefined {
      return values.get(agentId);
    },
    async load(agentId: string, refresh = false) {
      if (values.has(agentId) && !refresh) return;
      if (!loading.has(agentId))
        loading.set(
          agentId,
          (agentId === DRAFT
            ? client.rpc(draftReadRpc, {})
            : client.rpc(skillsReadRpc, { agentId })
          )
            .then(({ skills }) => {
              if (values.get(agentId)?.join() !== skills.join()) {
                values.set(agentId, skills);
                notify();
              }
            })
            .finally(() => loading.delete(agentId)),
        );
      await loading.get(agentId);
    },
    // Update after persistence so the label never shows a selection the hook cannot read.
    async set(agentId: string, skills: SkillId[]) {
      await loading.get(agentId);
      const saved =
        agentId === DRAFT
          ? await client.rpc(draftWriteRpc, { skills })
          : await client.rpc(skillsWriteRpc, { agentId, skills });
      values.set(agentId, saved.skills);
      notify();
    },
    subscribe(fn: () => void) {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
  };
}
export type SkillState = ReturnType<typeof createSkillState>;
