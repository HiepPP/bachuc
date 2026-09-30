import type { PluginButtonRegistration, PluginClientContext } from "@getpaseo/plugin/client";
import { boardRpc, removeRunRpc, type BoardRun } from "../shared/board";

export async function removeFinishedRun(
  run: Pick<BoardRun, "id" | "endedAt">,
  observingSince: string,
  remove: (scope: string) => Promise<{ removed: boolean }>,
  read: () => Promise<{
    observingSince: string;
    runs: readonly Pick<BoardRun, "id" | "status" | "endedAt">[];
  }>,
) {
  if ((await remove(observingSince)).removed) return true;
  const fresh = await read();
  const current = fresh.runs.find((item) => item.id === run.id);
  if (!current) return true;
  if (
    fresh.observingSince === observingSince ||
    current.status === "running" ||
    current.endedAt !== run.endedAt
  )
    return false;
  return (await remove(fresh.observingSince)).removed;
}

export function installRemoveButtons(
  client: PluginClientContext,
  openParent: (agentId: string) => void,
  /** Omitted when this client cannot open the project's new-thread screen. */
  startNewThread?: (run: BoardRun) => void,
) {
  const buttons = new Map<string, PluginButtonRegistration>();
  const newThreadButtons = new Map<string, PluginButtonRegistration>();
  const parentButtons = new Map<string, PluginButtonRegistration>();
  const workspaces = new Map<string, string>();
  let stopped = false;
  let revision = 0;
  let timer: ReturnType<typeof setTimeout>;
  const clear = () => {
    for (const button of buttons.values()) button.remove();
    buttons.clear();
    for (const button of newThreadButtons.values()) button.remove();
    newThreadButtons.clear();
    for (const button of parentButtons.values()) button.remove();
    parentButtons.clear();
    workspaces.clear();
  };
  async function refresh() {
    const currentRevision = revision;
    try {
      const snapshot = await client.rpc(boardRpc, {});
      const eligible = snapshot.runs.filter((run) => run.status !== "running" || run.parentAgentId);
      const entries = await Promise.all(
        eligible.map(async (run) => {
          let workspaceId = workspaces.get(run.id);
          if (!workspaceId) {
            const agent = await client.paseo.agents.ref(run.agentId).refresh();
            workspaceId = agent?.agent.workspaceId ?? undefined;
          }
          return { run, workspaceId };
        }),
      );
      if (stopped || currentRevision !== revision) return;
      const ids = new Set(
        entries
          .filter(({ run, workspaceId }) => workspaceId && run.status !== "running")
          .map(({ run }) => run.id),
      );
      const childIds = new Set(
        entries
          .filter(
            ({ run, workspaceId }) =>
              workspaceId && run.parentAgentId && run.parentAgentId !== run.agentId,
          )
          .map(({ run }) => run.id),
      );
      for (const [id, button] of parentButtons) {
        if (!childIds.has(id)) {
          button.remove();
          parentButtons.delete(id);
        }
      }
      for (const [id, button] of newThreadButtons) {
        if (!ids.has(id)) {
          button.remove();
          newThreadButtons.delete(id);
        }
      }
      for (const [id, button] of buttons) {
        if (!ids.has(id)) {
          button.remove();
          buttons.delete(id);
          workspaces.delete(id);
        }
      }
      for (const { run, workspaceId } of entries) {
        if (!workspaceId) continue;
        workspaces.set(run.id, workspaceId);
        if (childIds.has(run.id)) {
          const parentButton = {
            title: "Jump To Parent",
            label: "Jump To Parent",
            icon: "CornerLeftUp",
            behavior: {
              kind: "action" as const,
              onPress() {
                openParent(run.parentAgentId!);
              },
            },
          };
          const existing = parentButtons.get(run.id);
          if (existing) existing.update(parentButton);
          else
            parentButtons.set(
              run.id,
              client.addComposerPill({
                id: `parent-${run.id}`,
                workspaceId,
                agentId: run.agentId,
                button: parentButton,
              }),
            );
        }
        if (run.status === "running") continue;
        const remove = async () => {
          const removed = await removeFinishedRun(
            run,
            snapshot.observingSince,
            (scope) =>
              client.rpc(removeRunRpc, {
                id: run.id,
                observingSince: scope,
                endedAt: run.endedAt,
              }),
            () => client.rpc(boardRpc, {}),
          );
          if (!removed) throw new Error("Run changed. Refresh and retry.");
          revision++;
          for (const registry of [buttons, newThreadButtons]) {
            registry.get(run.id)?.remove();
            registry.delete(run.id);
          }
          workspaces.delete(run.id);
          return !stopped;
        };
        const button = {
          title: "Remove from Board",
          label: "Remove",
          icon: "X",
          behavior: {
            kind: "action" as const,
            async onPress() {
              if (await remove()) client.openSurface("board");
            },
          },
        };
        const existing = buttons.get(run.id);
        if (existing) existing.update(button);
        else {
          buttons.set(
            run.id,
            client.addComposerPill({
              id: `remove-${run.id}`,
              workspaceId,
              agentId: run.agentId,
              button,
            }),
          );
        }
        if (!startNewThread || run.cwd === undefined) continue;
        const newThreadButton = {
          title: "Remove and Start New Thread",
          label: "Remove & New Thread",
          icon: "SquarePen",
          behavior: {
            kind: "action" as const,
            async onPress() {
              if (await remove()) startNewThread(run);
            },
          },
        };
        const existingNewThread = newThreadButtons.get(run.id);
        if (existingNewThread) existingNewThread.update(newThreadButton);
        else
          newThreadButtons.set(
            run.id,
            client.addComposerPill({
              id: `new-thread-${run.id}`,
              workspaceId,
              agentId: run.agentId,
              button: newThreadButton,
            }),
          );
      }
    } catch {
      // Never offer Remove when the host cannot confirm finished Board membership.
      if (!stopped && currentRevision === revision) clear();
    } finally {
      if (!stopped) timer = setTimeout(refresh, 2_000);
    }
  }
  void refresh();
  return () => {
    stopped = true;
    clearTimeout(timer);
    clear();
  };
}
