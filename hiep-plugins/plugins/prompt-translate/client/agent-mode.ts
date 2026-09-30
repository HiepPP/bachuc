import { z } from "zod";
import type { PluginClientContext } from "@getpaseo/plugin/client";
import { modeReadRpc, modeWriteRpc } from "../shared/contracts";
import type { TranslateSettings } from "../shared/settings";
import { reactProps, type El } from "./dom";
export type Mode = TranslateSettings["cavemanMode"];
const agentIdSchema = z.string().uuid();
export function composerHost(node: El): string | null {
  const field = node.querySelector("[data-composer-input], textarea") ?? node;
  const props = reactProps(field, (value) => typeof value.voiceServerId === "string");
  return (props?.voiceServerId as string | undefined) ?? null;
}
export function composerAgent(node: El): string | null {
  const field = node.querySelector("[data-composer-input], textarea") ?? node;
  const props = reactProps(field, (value) => typeof value.voiceAgentId === "string");
  // New-thread composers expose a draft key before an agent exists.
  const id = agentIdSchema.safeParse(props?.voiceAgentId);
  return id.success ? id.data : null;
}
const drafts = new WeakMap<El, { raw: string; key: string }>();
let draftSequence = 0;
export function composerModeKey(node: El): string | null {
  const field = node.querySelector("[data-composer-input], textarea") ?? node;
  const agent = composerAgent(field);
  if (agent) return agent;
  const props = reactProps(field, (value) => typeof value.voiceAgentId === "string");
  if (typeof props?.voiceAgentId !== "string") return null;
  let draft = drafts.get(field);
  if (!draft || draft.raw !== props.voiceAgentId) {
    draft = { raw: props.voiceAgentId, key: `draft:${++draftSequence}` };
    drafts.set(field, draft);
  }
  return draft.key;
}
// Untouched new-thread drafts follow the host's configured mode; saved agents keep their own.
export function createAgentModes(
  client: Pick<PluginClientContext, "rpc">,
  draftMode: () => Mode = () => "follow-agent",
) {
  const values = new Map<string, Mode>();
  const loading = new Map<string, Promise<void>>();
  const listeners = new Set<() => void>();
  const notify = () => listeners.forEach((fn) => fn());
  return {
    get(id: string) {
      return values.get(id) ?? (id.startsWith("draft:") ? draftMode() : undefined);
    },
    async load(id: string, refresh = false) {
      if (id.startsWith("draft:")) return;
      if (values.has(id) && !refresh) return;
      if (!loading.has(id))
        loading.set(
          id,
          client
            .rpc(modeReadRpc, { agentId: id })
            .then(({ mode }) => {
              if (values.get(id) !== mode) {
                values.set(id, mode);
                notify();
              }
            })
            .finally(() => loading.delete(id)),
        );
      await loading.get(id);
    },
    async set(id: string, mode: Mode) {
      if (id.startsWith("draft:")) {
        values.set(id, mode);
        notify();
        return;
      }
      // Update after persistence: a send during saving must await this operation.
      const task = client.rpc(modeWriteRpc, { agentId: id, mode }).then(() => {
        values.set(id, mode);
        notify();
      });
      loading.set(id, task);
      try {
        await task;
      } finally {
        if (loading.get(id) === task) loading.delete(id);
      }
    },
    async ready(id: string) {
      await loading.get(id);
      await this.load(id, true);
      return this.get(id)!;
    },
    subscribe(fn: () => void) {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
  };
}
export type AgentModeState = ReturnType<typeof createAgentModes>;
