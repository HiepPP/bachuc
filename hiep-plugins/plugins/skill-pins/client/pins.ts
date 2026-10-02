import { useRpc, useSettings, type PluginButtonContext } from "@getpaseo/plugin/client";
import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import type { SkillId } from "../shared/catalog";
import { skillsReadRpc, skillsWriteRpc } from "../shared/contracts";
import { draftSkillsSchema } from "../shared/draft";
import { pinSettings } from "../shared/settings";

// The pill label and its menu are separate components; this keeps them on the same value.
const known = new Map<string, SkillId[]>();
const listeners = new Set<() => void>();

function publish(key: string, skills: SkillId[]) {
  const previous = known.get(key);
  // Settings hand back a new array each render; an equal value must not notify again.
  if (previous?.length === skills.length && previous.every((id, index) => id === skills[index]))
    return;
  known.set(key, skills);
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** One composer's pins: an agent's saved skills, or a New workspace draft's. */
export function useSkillPins(context: PluginButtonContext) {
  const agentId = context.context === "agent" ? context.agentId : undefined;
  const draft = context.context === "draft" ? context.draft : undefined;
  const key = agentId ?? draft?.id;
  const settings = useSettings(pinSettings);
  const defaults = settings.status === "ready" ? settings.values.defaults : undefined;
  const read = useRpc(skillsReadRpc);
  const write = useRpc(skillsWriteRpc);
  const skills = useSyncExternalStore(subscribe, () => (key ? known.get(key) : undefined)) ?? null;
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (draft) {
      const saved = draftSkillsSchema.safeParse(draft.get());
      if (saved.success) publish(draft.id, saved.data.skills);
      else if (defaults) publish(draft.id, defaults);
      return;
    }
    if (!agentId) return;
    let active = true;
    read({ agentId })
      .then((value) => active && publish(agentId, value.skills))
      .catch((failure: unknown) => active && setError(String(failure)));
    return () => {
      active = false;
    };
  }, [agentId, draft, defaults, read]);
  const save = useCallback(
    (next: SkillId[]) => {
      if (!key || !known.has(key)) return;
      publish(key, next);
      if (draft) {
        draft.set({ skills: next });
        return;
      }
      write({ agentId: key, skills: next })
        .then((saved) => publish(key, saved.skills))
        .catch((failure: unknown) => setError(String(failure)));
    },
    [key, draft, write],
  );
  return { skills, error, save };
}
