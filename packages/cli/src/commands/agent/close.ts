import type { Command } from "commander";
import { connectToDaemon, resolveAgentId } from "../../utils/client.js";
import type {
  CommandError,
  CommandOptions,
  OutputSchema,
  SingleResult,
} from "../../output/index.js";

interface AgentCloseResult {
  agentId: string;
  status: "closed";
}

const closeSchema: OutputSchema<AgentCloseResult> = {
  idField: "agentId",
  columns: [
    { header: "AGENT ID", field: "agentId" },
    { header: "STATUS", field: "status" },
  ],
};

export async function runCloseCommand(
  agentIdArg: string,
  options: CommandOptions,
  _command: Command,
): Promise<SingleResult<AgentCloseResult>> {
  const client = await connectToDaemon({ target: options.daemonTarget });

  try {
    const payload = await client.fetchAgents({ filter: { includeArchived: true } });
    const agentId = resolveAgentId(
      agentIdArg,
      payload.entries.map((entry) => entry.agent),
    );
    if (!agentId) {
      throw {
        code: "AGENT_NOT_FOUND",
        message: `Agent not found: ${agentIdArg}`,
      } satisfies CommandError;
    }
    await client.closeAgentRuntime(agentId);
    return { type: "single", data: { agentId, status: "closed" }, schema: closeSchema };
  } finally {
    await client.close().catch(() => undefined);
  }
}
