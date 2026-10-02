import type { PaseoApi } from "@getpaseo/client";
import type { Scope } from "../shared/contracts";
import type { Current, Driver } from "./engine";

// Enough for every thread a client can keep on screen; older entries are read again on demand.
const KEPT_THREADS = 200;

export function createDriver(getApi: () => PaseoApi, serverId: string): Driver {
  async function agent(scope: Scope) {
    if (scope.serverId !== serverId) throw new Error("Wrong host.");
    const handle = getApi().agents.ref(scope.agentId);
    await handle.refresh();
    if (handle.workspaceId !== scope.workspaceId || handle.archivedAt || !handle.current())
      throw new Error("Conversation unavailable.");
    return handle;
  }
  // Reading a timeline resumes a closed runtime, so the panel's poll would undo every close. A
  // closed thread cannot change, so its latest turn is served from the last read instead.
  const latestTurns = new Map<string, Current>();
  return {
    async read(scope) {
      const handle = await agent(scope);
      const kept = handle.status === "closed" ? latestTurns.get(scope.agentId) : undefined;
      // A closed thread is at rest: sending resumes it.
      if (kept) return { ...kept, busy: false };
      const page = await handle.timeline.refetch({ direction: "tail", limit: 1000 });
      if (page.error || page.gap || page.hasNewer) throw new Error("Timeline is incomplete.");
      const current = {
        busy:
          !["idle"].includes(handle.status ?? "") ||
          !!handle.activeTurn ||
          !!handle.pendingPermissions?.length,
        epoch: page.epoch,
        // A contiguous tail containing a user boundary includes the entire latest turn.
        // Older turns are not required to validate its suggested continuation.
        complete:
          !page.hasOlder || page.entries.some((entry) => entry.item.type === "user_message"),
        rows: page.entries.map((entry) => ({
          type: entry.item.type,
          text:
            "text" in entry.item && typeof entry.item.text === "string"
              ? entry.item.text
              : undefined,
          id: `${entry.seqStart}:${entry.seqEnd}`,
          timestamp: Date.parse(entry.timestamp),
          messageId:
            entry.item.type === "user_message"
              ? (entry.item.clientMessageId ?? entry.item.messageId)
              : undefined,
        })),
      };
      const boundary = current.rows.findLastIndex((row) => row.type === "user_message");
      latestTurns.delete(scope.agentId);
      latestTurns.set(scope.agentId, {
        ...current,
        rows: boundary < 0 ? [] : current.rows.slice(boundary),
      });
      if (latestTurns.size > KEPT_THREADS) latestTurns.delete(latestTurns.keys().next().value!);
      return current;
    },
    async send(scope, text, messageId, canSend) {
      const handle = await agent(scope);
      if (
        (handle.status !== "idle" && handle.status !== "closed") ||
        handle.activeTurn ||
        handle.pendingPermissions?.length
      )
        throw new Error("Agent is busy.");
      if (!canSend()) throw new Error("Send cancelled.");
      await handle.send(text, { messageId });
    },
    async start(scope, text, id) {
      const handle = await agent(scope);
      const source = handle.current()!;
      if (!handle.cwd) throw new Error("Conversation directory unavailable.");
      await getApi().agents.create({
        cwd: handle.cwd,
        config: {
          provider: source.model ? `${source.provider}/${source.model}` : source.provider,
          ...(source.currentModeId ? { modeId: source.currentModeId } : {}),
          ...(source.thinkingOptionId ? { thinkingOptionId: source.thinkingOptionId } : {}),
        },
        prompt: text,
        // A repeated click or reconnect must not create a second conversation.
        idempotencyKey: id,
      });
    },
  };
}
