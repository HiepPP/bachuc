import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const client = new Client({ name: "jev-smoke", version: "0.1.0" });
const transport = new StdioClientTransport({
  command: process.execPath,
  args: [fileURLToPath(new URL("./mcp.mjs", import.meta.url))],
  env: {
    ELECTRON_RUN_AS_NODE: "1",
    PASEO_JEV_CONFIG_PATH:
      process.env.PASEO_JEV_CONFIG_PATH || path.join(homedir(), ".paseo", "config.json"),
  },
});

try {
  await client.connect(transport);
  const tools = await client.listTools();
  if (!tools.tools.some((tool) => tool.name === "jev_evaluate")) {
    throw new Error("MCP did not expose jev_evaluate.");
  }
  const result = await client.callTool({
    name: "jev_evaluate",
    arguments: {
      state: "The backend unit tests passed. A database migration still needs human review.",
      questions: {
        testsPassed: { type: "boolean", instructions: "Did the unit tests pass?" },
        nextStep: {
          type: "choice",
          instructions: "Choose the next action.",
          criteria: { review: "Review the pending migration", finish: "All work is complete" },
        },
        risk: {
          type: "score",
          instructions: "Rate unresolved release risk.",
          criteria: ["No outstanding work", "Review required", "Known failing tests"],
        },
      },
    },
  });
  console.log(JSON.stringify(result, null, 2));
  if (result.isError) process.exitCode = 1;
} finally {
  await client.close();
  await transport.close();
}
