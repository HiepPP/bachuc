import type { PaseoApi } from "@getpaseo/client";
import { randomUUID } from "node:crypto";
import { realpath } from "node:fs/promises";
import type { Driver, Job, Profile } from "./types";
import { readAgentUsage } from "./agent-usage";

export function createDriver(getApi: () => PaseoApi): Driver {
  return {
    async profiles(cwd) {
      const api = getApi();
      const [{ config }, available] = await Promise.all([
        api.config.get(),
        api.providers.listAvailable(),
      ]);
      const configured = (config.agentProfiles ?? []).filter(
        (p) => p.model && available.providers.some((a) => a.provider === p.provider && a.available),
      );
      const result: Profile[] = [];
      for (const provider of new Set(configured.map((p) => p.provider))) {
        const [models, modes] = await Promise.all([
          api.providers.listModels(provider, { cwd }),
          api.providers.listModes(provider, { cwd }),
        ]);
        if (models.error || modes.error) continue;
        for (const p of configured.filter((p) => p.provider === provider)) {
          const model = models.models?.find(
            (m) => m.id === p.model || m.aliases?.includes(p.model!),
          );
          if (
            !model ||
            model.isSelectable === false ||
            (p.modeId && !modes.modes?.some((m) => m.id === p.modeId)) ||
            (p.thinkingOptionId && !model.thinkingOptions?.some((t) => t.id === p.thinkingOptionId))
          )
            continue;
          result.push({
            id: p.id,
            name: p.name,
            provider: p.provider,
            model: p.model!,
            modeId: p.modeId,
            thinkingOptionId: p.thinkingOptionId,
            featureValues: p.featureValues,
            notes: p.notes,
          });
        }
      }
      return result;
    },
    async launch(job, profile, phase, prompt) {
      const api = getApi();
      const parent = api.agents.ref(job.parentId);
      await parent.refresh();
      if (
        !parent.workspaceId ||
        !parent.cwd ||
        (await realpath(parent.cwd)) !== job.cwd ||
        parent.archivedAt
      )
        throw new Error("Parent workspace changed or unavailable.");
      const child = await api.workspaces.ref(parent.workspaceId).agents.create({
        parent: job.parentId,
        config: {
          provider: `${profile.provider}/${profile.model}`,
          modeId: profile.modeId,
          thinkingOptionId: profile.thinkingOptionId,
          featureValues: profile.featureValues,
        },
        title: `Jev: ${job.task.id} / ${phase}`,
        env: { PASEO_ORCH_CHILD: "1" },
        prompt,
        labels: { "jev-orchestrator": "child", "jev-job": job.key, "jev-phase": phase },
        requestId: randomUUID(),
      });
      return child.id;
    },
    async wait(id, timeoutMs) {
      const child = getApi().agents.ref(id);
      const result = await child.waitForFinish(timeoutMs);
      const usage = result.status === "timeout" ? undefined : await readAgentUsage(child.current());
      return {
        usage,
        status: result.status,
        output: result.lastMessage ?? result.error ?? "",
        costUsd: child.lastUsage?.totalCostUsd,
        observed: child.runtimeInfo
          ? {
              provider: child.runtimeInfo.provider,
              model: child.runtimeInfo.model,
              thinkingOptionId: child.runtimeInfo.thinkingOptionId,
              modeId: child.runtimeInfo.modeId,
            }
          : undefined,
        tokens: usage?.complete ? (usage.totalTokens ?? undefined) : undefined,
      };
    },
    async archive(id) {
      const child = getApi().agents.ref(id);
      const current = await child.refresh();
      if (current && !child.archivedAt) await child.archive();
    },
    async notify(job: Job) {
      const parent = getApi().agents.ref(job.parentId);
      await parent.timeline.append({
        type: "plugin",
        id: `jev-job-${job.task.id}`,
        kind: "job",
        version: 1,
        data: {
          id: job.task.id,
          status: job.status,
          message: job.message,
          children: job.attempts.map((a) => a.childId),
        },
      });
      await parent.refresh();
      if (!parent.archivedAt && parent.status === "idle")
        await parent.send(
          `[Jev orchestrator result] Task ${job.task.id}: ${job.status}. ${job.message} Read orchestrator_status for child evidence. This notification does not authorize more work.`,
        );
    },
  };
}
