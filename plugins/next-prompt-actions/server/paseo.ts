import type { PaseoApi } from "@getpaseo/client";
import type { Scope } from "../shared/contracts";
import type { Driver } from "./engine";

export function createDriver(getApi: () => PaseoApi, serverId: string): Driver {
  async function agent(scope: Scope) {
    if (scope.serverId !== serverId) throw new Error("Wrong host.");
    const handle = getApi().agents.ref(scope.agentId);
    await handle.refresh();
    if (handle.workspaceId !== scope.workspaceId || handle.archivedAt || !handle.current())
      throw new Error("Conversation unavailable.");
    return handle;
  }
  return {
    async read(scope) {
      const handle = await agent(scope);
      const page = await handle.timeline.refetch({ direction: "tail", limit: 1000 });
      if (page.error || page.gap || page.hasNewer) throw new Error("Timeline is incomplete.");
      return {
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
    },
    async send(scope, text, messageId, canSend) {
      const handle = await agent(scope);
      if (handle.status !== "idle" || handle.activeTurn || handle.pendingPermissions?.length)
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
