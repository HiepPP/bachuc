import { useRpc, useSettings, type PluginButtonContext } from "@getpaseo/plugin/client";
import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import { modeReadRpc, modeWriteRpc } from "../shared/contracts";
import { draftPrefsSchema } from "../shared/draft";
import { translateSettings } from "../shared/settings";
import type { Mode } from "./modes";

export type Prefs = { mode: Mode; rewrite: boolean };

// The pill label and its popover are separate components; this keeps them on the same value.
const known = new Map<string, Prefs>();
const listeners = new Set<() => void>();

function publish(key: string, prefs: Prefs) {
  known.set(key, prefs);
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** One composer's choices: an agent's saved prefs, or a New workspace draft's. */
export function usePromptPrefs(context: PluginButtonContext) {
  const agentId = context.context === "agent" ? context.agentId : undefined;
  const draft = context.context === "draft" ? context.draft : undefined;
  const key = agentId ?? draft?.id;
  const settings = useSettings(translateSettings);
  const defaults = settings.status === "ready" ? settings.values : undefined;
  const read = useRpc(modeReadRpc);
  const write = useRpc(modeWriteRpc);
  const prefs = useSyncExternalStore(subscribe, () => (key ? known.get(key) : undefined)) ?? null;
  const [error, setError] = useState<string | null>(null);
  const defaultMode = defaults?.cavemanMode;
  const defaultRewrite = defaults?.enhanceShortcut;
  useEffect(() => {
    if (draft) {
      const saved = draftPrefsSchema.safeParse(draft.get());
      if (saved.success) publish(draft.id, saved.data);
      else if (defaultMode !== undefined && defaultRewrite !== undefined)
        publish(draft.id, { mode: defaultMode, rewrite: defaultRewrite });
      return;
    }
    if (!agentId) return;
    let active = true;
    read({ agentId })
      .then((value) => active && publish(agentId, value))
      .catch((failure: unknown) => active && setError(String(failure)));
    return () => {
      active = false;
    };
  }, [agentId, draft, defaultMode, defaultRewrite, read]);
  const save = useCallback(
    (patch: Partial<Prefs>) => {
      const current = key ? known.get(key) : undefined;
      if (!key || !current) return;
      const next = { ...current, ...patch };
      publish(key, next);
      if (draft) {
        draft.set(next);
        return;
      }
      write({ agentId: key, ...patch })
        .then((saved) => publish(key, saved))
        .catch((failure: unknown) => setError(String(failure)));
    },
    [key, draft, write],
  );
  return { prefs, error, save };
}
