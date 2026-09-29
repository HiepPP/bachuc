import type { SkillId } from "../shared/catalog";
import {
  ROOT,
  composerAgent,
  reactProps,
  type Doc,
  type DomEvent,
  type El,
  type Observer,
} from "./dom";

export type QueueApi = {
  skills(agentId: string): SkillId[] | undefined;
  prepare(input: { agentId: string; text: string; skills: SkillId[] }): Promise<{ token: string }>;
  bindQueue(agentId: string, token: string, queueId: string): Promise<unknown>;
  cancelQueue(agentId: string, queueId: string): Promise<unknown>;
};

type Row = { id: string; text: string; button: El; editLabel: string; agentId: string | null };

// Direct sends need no snapshot: the hook reads skills.json, which each toggle saves.
// Only a queued prompt can run after the selection changes, so each new queue row gets a
// snapshot of the selection at the moment it appeared. No key or send event is intercepted.
export function installQueueSnapshots(
  api: QueueApi,
  doc: Doc,
  Observer: Observer | undefined,
  owns: (root: El) => boolean = () => true,
) {
  const seen = new Set<string>();
  let primed = false;
  let scheduled = false;
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const bindings = new Map<string, Promise<unknown>>();

  function rows() {
    const found = new Map<string, Row>();
    for (const button of Array.from(doc.querySelectorAll('button,[role="button"]'))) {
      const props = reactProps(
        button,
        (p) =>
          typeof p.onEdit === "function" &&
          typeof p.onSendNow === "function" &&
          typeof p.editLabel === "string",
      );
      const item = props?.item as { id?: string; text?: string } | undefined;
      if (typeof item?.id !== "string" || typeof item.text !== "string") continue;
      const root = button.closest(ROOT);
      const agentId = root && owns(root) ? composerAgent(root) : null;
      if (!found.has(item.id))
        found.set(item.id, {
          id: item.id,
          text: item.text,
          button,
          editLabel: props!.editLabel as string,
          agentId,
        });
    }
    return found;
  }

  function scan() {
    scheduled = false;
    if (stopped) return;
    const current = rows();
    for (const row of current.values()) {
      if (seen.has(row.id)) continue;
      seen.add(row.id);
      // Rows that existed before this plugin loaded keep whatever the hook reads later.
      if (!primed || !row.agentId || !row.text.trim()) continue;
      const skills = api.skills(row.agentId);
      if (!skills) continue;
      const agentId = row.agentId;
      const binding = api
        .prepare({ agentId, text: row.text, skills })
        .then(({ token }) => api.bindQueue(agentId, token, row.id));
      bindings.set(row.id, binding);
      void binding.catch((error) => console.warn("[skill-pins] Queue snapshot failed", error));
    }
    for (const id of seen) if (!current.has(id)) bindings.delete(id);
    primed = true;
  }

  // Editing a queued prompt puts it back in the draft; drop its snapshot so a resend
  // captures the selection at that time. The host click is never blocked.
  function onClick(event: DomEvent) {
    const button = (event.target as El | null)?.closest?.('button,[role="button"]');
    if (!button) return;
    for (const row of rows().values()) {
      if (row.button !== button || button.getAttribute("aria-label") !== row.editLabel) continue;
      if (!row.agentId) return;
      const agentId = row.agentId;
      void (bindings.get(row.id) ?? Promise.resolve())
        .catch(() => undefined)
        .then(() => api.cancelQueue(agentId, row.id))
        .catch((error) => console.warn("[skill-pins] Queue cancel failed", error));
      return;
    }
  }

  doc.addEventListener("click", onClick, true);
  const observer = Observer
    ? new Observer(() => {
        if (scheduled || stopped) return;
        scheduled = true;
        timer = setTimeout(scan, 0);
      })
    : undefined;
  observer?.observe(doc.body, { childList: true, subtree: true });
  scan();
  return {
    scan,
    onClick,
    stop() {
      stopped = true;
      observer?.disconnect();
      clearTimeout(timer);
      doc.removeEventListener("click", onClick, true);
    },
  };
}
