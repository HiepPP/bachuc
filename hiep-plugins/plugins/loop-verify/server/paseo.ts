import type { PaseoApi } from "@getpaseo/client";
import { STATUS_KIND, STATUS_VERSION } from "../shared/status";
import type { Driver } from "./engine";

export function createDriver(api: () => PaseoApi): Driver {
  return {
    async inspect(agentId) {
      const ref = api().agents.ref(agentId);
      await ref.refresh();
      const agent = ref.current();
      if (!agent) return null;
      return {
        id: agent.id,
        cwd: agent.cwd,
        provider: agent.provider,
        model: agent.model,
        modeId: agent.currentModeId,
        thinkingOptionId: agent.thinkingOptionId ?? null,
        labels: agent.labels ?? {},
      };
    },
    async create(request) {
      await api().agents.create({
        agentId: request.agentId,
        parent: request.parentId,
        cwd: request.cwd,
        config: request.config,
        title: request.title,
        prompt: request.prompt,
        labels: request.labels,
      });
    },
    async publish(agentId, rowId, status) {
      await api().agents.ref(agentId).timeline.append({
        type: "plugin",
        id: rowId,
        kind: STATUS_KIND,
        version: STATUS_VERSION,
        data: status,
      });
    },
  };
}
