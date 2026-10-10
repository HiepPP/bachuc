import { useMemo } from "react";
import type { PluginButtonContext } from "@getpaseo/plugin/client";

type Draft = Extract<PluginButtonContext, { context: "draft" }>["draft"];

/** Owned by one New workspace draft and host; writes are synchronous before Send. */
export class PluginComposerDraftState {
  private values: Record<string, unknown> = {};
  constructor(readonly id: string) {}

  forPlugin(pluginId: string, cwd?: string): Draft {
    return {
      id: this.id,
      cwd,
      get: () => this.snapshot()[pluginId],
      set: (value) => {
        this.values[pluginId] = JSON.parse(JSON.stringify(value));
      },
    };
  }

  snapshot = (): Record<string, unknown> => JSON.parse(JSON.stringify(this.values));

  environment(): Record<string, string> {
    return Object.fromEntries(
      Object.entries(this.values).map(([id, value]) => [
        `PASEO_PLUGIN_COMPOSER_${id}`,
        JSON.stringify(value),
      ]),
    );
  }
}

export function usePluginComposerDraft(
  serverId: string,
  draftId: string,
): PluginComposerDraftState {
  const drafts = useMemo(
    () => ({ id: draftId, hosts: new Map<string, PluginComposerDraftState>() }),
    [draftId],
  );
  let draft = drafts.hosts.get(serverId);
  if (!draft) {
    draft = new PluginComposerDraftState(`${serverId}:${drafts.id}`);
    drafts.hosts.set(serverId, draft);
  }
  return draft;
}
