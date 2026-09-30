import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { z } from "zod";
import { reportSchema } from "../shared/preflight";
import { runPreflight } from "./preflight";

export function createPreflightServer(workspace: string) {
  if (!path.isAbsolute(workspace)) throw new Error("Absolute creating workspace required.");
  const server = new McpServer({ name: "workspace-preflight", version: "0.2.0" });
  server.registerTool(
    "workspace_preflight",
    {
      description:
        "Read bounded prerequisite evidence for the creating workspace. No directory overrides. Measurements do not prove task coverage. Collect required checks, then optionally ask jev_evaluate with sanitized evidence; never execute repairs automatically.",
      inputSchema: z.strictObject({}),
      outputSchema: reportSchema,
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: true,
      },
    },
    async () => {
      const report = await runPreflight(workspace);
      return {
        content: [{ type: "text", text: JSON.stringify(report) }],
        structuredContent: report,
      };
    },
  );
  return server;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const workspace = process.env.PASEO_PREFLIGHT_WORKSPACE;
  if (!workspace || !path.isAbsolute(workspace)) throw new Error("Missing workspace binding.");
  await createPreflightServer(workspace).connect(new StdioServerTransport());
}
