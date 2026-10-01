import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { expect, test } from "vitest";
import { DaemonClient } from "../test-utils/daemon-client.js";
import { createTestPaseoDaemon } from "../test-utils/paseo-daemon.js";

async function connectMcp(port: number, callerAgentId: string): Promise<Client> {
  const client = new Client({ name: "plugin-tools-test", version: "1.0.0" });
  await client.connect(
    new StreamableHTTPClientTransport(
      new URL(`http://127.0.0.1:${port}/mcp/agents?callerAgentId=${callerAgentId}`),
    ),
  );
  return client;
}

test("the daemon MCP server serves plugin tools with the caller agent", async () => {
  const directory = await mkdtemp(path.join(tmpdir(), "paseo-plugin-tools-"));
  const daemon = await createTestPaseoDaemon({ daemonVersion: "0.8.0" });
  const client = new DaemonClient({ url: `ws://127.0.0.1:${daemon.port}/ws`, appVersion: "0.8.0" });
  const baseline = await connectMcp(daemon.port, "caller-1");
  let mcp: Client | null = null;
  try {
    const builtInNames = (await baseline.listTools()).tools.map((tool) => tool.name).sort();
    await writeFile(
      path.join(directory, "paseo-plugin.json"),
      JSON.stringify({ id: "tools", requirements: { paseo: ">=0.8.0" } }),
    );
    await writeFile(
      path.join(directory, "index.server.ts"),
      `
import { z } from "zod";
export default function contribute(server) {
  server.registerTool({
    name: "echo_caller",
    description: "Echo the input and the caller.",
    inputSchema: z.object({ text: z.string() }),
    handler: ({ text }, { callerAgentId }) => ({
      text: text + ":" + callerAgentId,
      structured: { text, callerAgentId },
    }),
  });
  server.registerTool({
    name: "fail",
    description: "Always fails.",
    inputSchema: z.object({}),
    handler: () => {
      throw new Error("Tool broke");
    },
  });
  server.registerTool({
    name: "list_agents",
    description: "Clashes with a built-in tool.",
    inputSchema: z.object({}),
    handler: () => ({ text: "plugin" }),
  });
  return () => {};
}
`,
    );
    await client.connect();
    await client.patchDaemonConfig({ pluginsEnabled: true });
    await client.installDirectoryPlugin(directory);
    mcp = await connectMcp(daemon.port, "caller-1");
    const names = (await mcp.listTools()).tools.map((tool) => tool.name).sort();
    expect(names).toEqual([...builtInNames, "echo_caller", "fail"].sort());
    await expect(
      mcp.callTool({ name: "echo_caller", arguments: { text: "hi" } }),
    ).resolves.toMatchObject({
      content: [{ type: "text", text: "hi:caller-1" }],
      structuredContent: { text: "hi", callerAgentId: "caller-1" },
    });
    await expect(mcp.callTool({ name: "fail", arguments: {} })).resolves.toMatchObject({
      isError: true,
      content: [{ type: "text", text: expect.stringContaining("Tool broke") }],
    });
  } finally {
    await mcp?.close();
    await baseline.close();
    await client.close();
    await daemon.close();
    await rm(directory, { recursive: true, force: true });
  }
}, 60_000);
