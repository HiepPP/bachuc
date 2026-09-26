import type { PluginHookAgent, PluginTurnOutcome } from "@getpaseo/plugin/server";
import { z } from "zod";
import { runSchema } from "../shared/board";
import type { BoardRun } from "../shared/board";
import { isCommandTitle } from "./title";

const persistedRunSchema = runSchema.extend({
  cwd: z.string(),
  providerTurnId: z.string().nullable(),
  snapshotTurnId: z.string().nullable(),
  dismissed: z.boolean().optional(),
  projectResolved: z.boolean(),
  // Goal from the first prompt when the host title is a Caveman command; null when unavailable.
  promptTitle: z.string().nullable().optional(),
});
export const runStateSchema = z.object({
  version: z.literal(1),
  active: z.array(persistedRunSchema.refine((run) => run.status === "running")),
  finished: z.array(persistedRunSchema.refine((run) => run.status !== "running")).max(50),
});
export type RunState = z.infer<typeof runStateSchema>;
type Run = RunState["active"][number];
export type ActiveAgent = PluginHookAgent & {
  project?: string;
  projectKey?: string;
  model?: string | null;
  effort?: string | null;
  pendingPermissions?: readonly unknown[];
  attentionReason?: "finished" | "error" | "permission" | null;
  activeTurn?: { turnId: string; startedAt: string | null } | null;
};

export function createRunStore(now = () => new Date().toISOString()) {
  const observingSince = now();
  const active = new Map<string, Run>();
  const finished: Run[] = [];
  let revision = 0;
  const metadata = (agent: PluginHookAgent) => ({
    agentId: agent.id,
    parentAgentId: agent.parentAgentId,
    title: agent.title || "Untitled run",
    project: agent.cwd.split(/[\\/]/).filter(Boolean).pop() || "Unknown project",
    provider: agent.provider,
    projectKey: `cwd:${agent.cwd}`,
    cwd: agent.cwd,
    projectResolved: false,
  });
  function make(agent: PluginHookAgent, turnId: string | null, startedAt: string | null): Run {
    const previous = finished.findIndex((run) => run.agentId === agent.id);
    const title = agent.title || active.get(agent.id)?.title || finished[previous]?.title;
    const prior = active.get(agent.id) ?? finished[previous];
    const starred = prior?.starred ?? false;
    if (previous !== -1) finished.splice(previous, 1);
    return {
      ...metadata(agent),
      ...(prior?.cwd === agent.cwd
        ? {
            project: prior.project,
            projectKey: prior.projectKey,
            projectResolved: prior.projectResolved,
          }
        : {}),
      starred,
      model: prior?.model,
      effort: prior?.effort,
      promptTitle: prior?.promptTitle,
      needsInput: false,
      title: title || "Untitled run",
      id: agent.id,
      providerTurnId: turnId,
      snapshotTurnId: null,
      status: "running",
      startedAt,
      endedAt: null,
    };
  }
  function display(run: Pick<Run, "title" | "promptTitle">) {
    return run.promptTitle && isCommandTitle(run.title) ? run.promptTitle : run.title;
  }
  function finish(run: Run, status: BoardRun["status"]) {
    run.status = status;
    run.needsInput = false;
    run.endedAt = now();
    if (active.get(run.agentId) === run) active.delete(run.agentId);
    finished.unshift(run);
    finished.splice(50);
  }
  return {
    get revision() {
      return revision;
    },
    restore(state: RunState) {
      active.clear();
      finished.length = 0;
      for (const run of state.active) active.set(run.agentId, run);
      finished.push(...state.finished);
    },
    exportState(): RunState {
      return { version: 1, active: [...active.values()], finished: [...finished] };
    },
    unresolvedProjects() {
      return [...active.values(), ...finished]
        .filter((run) => !run.dismissed && !run.projectResolved)
        .map((run) => ({ agentId: run.agentId, cwd: run.cwd }));
    },
    updateProject(
      agentId: string,
      cwd: string,
      project: { projectName: string; projectKey: string } | null,
    ) {
      const run = active.get(agentId) ?? finished.find((item) => item.agentId === agentId);
      if (!run || run.cwd !== cwd) return;
      if (project) {
        run.project = project.projectName;
        run.projectKey = `project:${project.projectKey}`;
      }
      run.projectResolved = true;
    },
    updateTitle(agentId: string, title: string | null) {
      if (!title) return;
      const run = active.get(agentId) ?? finished.find((item) => item.agentId === agentId);
      if (run) run.title = title;
    },
    pendingPromptTitles() {
      return [...active.values(), ...finished]
        .filter(
          (run) => !run.dismissed && run.promptTitle === undefined && isCommandTitle(run.title),
        )
        .map((run) => ({ agentId: run.agentId, title: run.title }));
    },
    setPromptTitle(agentId: string, title: string, promptTitle: string | null) {
      const run = active.get(agentId) ?? finished.find((item) => item.agentId === agentId);
      if (run?.title === title) run.promptTitle = promptTitle;
    },
    displayTitle(agentId: string) {
      const run = active.get(agentId) ?? finished.find((item) => item.agentId === agentId);
      return run && display(run);
    },
    start(agent: PluginHookAgent, turnId: string | null) {
      revision++;
      const existing = active.get(agent.id);
      if (
        existing &&
        ((existing.providerTurnId ?? existing.snapshotTurnId) === null ||
          (existing.providerTurnId ?? existing.snapshotTurnId) === turnId)
      ) {
        existing.parentAgentId = agent.parentAgentId;
        existing.providerTurnId = turnId;
        return;
      }
      active.set(agent.id, make(agent, turnId, now()));
    },
    end(agent: PluginHookAgent, turnId: string | null, outcome: PluginTurnOutcome) {
      revision++;
      let run = active.get(agent.id);
      const knownId = run?.providerTurnId ?? run?.snapshotTurnId;
      if (
        run &&
        turnId !== null &&
        knownId !== null &&
        knownId !== undefined &&
        knownId !== turnId
      ) {
        // A delayed terminal event must not finish a newer provider turn.
        return;
      }
      if (!run) {
        const previous = finished.find((r) => r.agentId === agent.id);
        if (previous) {
          if ((previous.providerTurnId ?? previous.snapshotTurnId) !== turnId) return;
          if (previous.status === "unknown") {
            previous.status = outcome.kind === "canceled" ? "cancelled" : outcome.kind;
            previous.endedAt = now();
            finished.splice(finished.indexOf(previous), 1);
            finished.unshift(previous);
          }
          return;
        }
        run = make(agent, turnId, null);
      }
      run.parentAgentId = agent.parentAgentId;
      run.providerTurnId = turnId;
      run.title = agent.title || run.title;
      finish(run, outcome.kind === "canceled" ? "cancelled" : outcome.kind);
    },
    reconcile(agents: ActiveAgent[], expectedRevision: number) {
      // Discard a list captured across a lifecycle event; the next refresh retries.
      if (revision !== expectedRevision) return;
      const present = new Set(agents.map((a) => a.id));
      for (const run of active.values()) if (!present.has(run.agentId)) finish(run, "unknown");
      for (const agent of agents) {
        let run = active.get(agent.id);
        const snapshotId = agent.activeTurn?.turnId ?? null;
        if (
          !run &&
          snapshotId &&
          finished.some(
            (r) => r.agentId === agent.id && (r.providerTurnId ?? r.snapshotTurnId) === snapshotId,
          )
        )
          continue;
        const knownId = run?.snapshotTurnId ?? run?.providerTurnId;
        if (run && knownId && snapshotId && knownId !== snapshotId) {
          run = undefined;
        }
        if (!run) {
          run = make(agent, null, agent.activeTurn?.startedAt ?? null);
          active.set(agent.id, run);
        }
        run.needsInput =
          (agent.pendingPermissions?.length ?? 0) > 0 || agent.attentionReason === "permission";
        run.parentAgentId = agent.parentAgentId;
        run.snapshotTurnId = snapshotId;
        run.startedAt = agent.activeTurn?.startedAt ?? run.startedAt;
        run.title = agent.title || run.title;
        run.project = agent.project || run.project;
        if (agent.projectKey) run.projectKey = `project:${agent.projectKey}`;
        run.projectResolved = true;
        if (agent.model !== undefined) {
          run.model = agent.model;
          run.effort = agent.effort ?? null;
        }
      }
    },
    /** Runs whose model was never read, such as subagents that finished between polls. */
    unresolvedModels() {
      return [...active.values(), ...finished]
        .filter((run) => !run.dismissed && run.model === undefined)
        .map((run) => run.agentId);
    },
    updateModel(agentId: string, model: string | null, effort: string | null) {
      const run = active.get(agentId) ?? finished.find((item) => item.agentId === agentId);
      if (!run || run.model !== undefined) return;
      run.model = model;
      run.effort = effort;
    },
    setStarred(id: string, scope: string, starred: boolean) {
      if (scope !== observingSince) return false;
      const run = active.get(id) ?? finished.find((item) => item.id === id);
      if (!run || run.dismissed) return false;
      run.starred = starred;
      return true;
    },
    removeFinished(id: string, scope: string, endedAt: string | null) {
      if (scope !== observingSince) return false;
      const run = finished.find((item) => item.id === id);
      if (!run || run.endedAt !== endedAt) return false;
      // Retain bounded metadata so repeated terminal events cannot restore a removed card.
      run.dismissed = true;
      // Finished subagents leave with their parent; a running subagent keeps its own subtree.
      const visible = finished.filter((item) => !item.dismissed);
      const queue = [run.agentId];
      const seen = new Set(queue);
      while (queue.length) {
        const parentId = queue.shift()!;
        for (const child of visible) {
          if (child.parentAgentId !== parentId || seen.has(child.agentId)) continue;
          seen.add(child.agentId);
          child.dismissed = true;
          queue.push(child.agentId);
        }
      }
      return true;
    },
    snapshot() {
      const runs = [...active.values(), ...finished]
        .filter((run) => !run.dismissed)
        .map(
          ({
            providerTurnId: _p,
            snapshotTurnId: _s,
            dismissed: _d,
            projectResolved: _r,
            promptTitle: _t,
            ...run
          }) => ({ ...run, title: display({ ...run, promptTitle: _t }) }),
        );
      return { runs, observingSince };
    },
    clear() {
      active.clear();
      finished.length = 0;
    },
  };
}
