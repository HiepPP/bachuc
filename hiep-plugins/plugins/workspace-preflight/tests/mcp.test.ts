import assert from "node:assert/strict";
import { test } from "node:test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { injectPreflight } from "../server/injection";
import { reportSchema } from "../shared/preflight";
import { fixture } from "./fixtures";
const plugin = fileURLToPath(new URL("..", import.meta.url));

test("new-agent injection preserves entries and coexists with Jev on built-in providers", () => {
  for (const provider of ["codex", "claude", "opencode"]) {
    const request = {
      config: {
        provider,
        cwd: "/workspace",
        mcpServers: { jev: { type: "stdio" as const, command: "node", args: ["jev.mjs"] } },
      },
    };
    const result = injectPreflight(request, plugin, "/workspace");
    assert.deepEqual(result.config.mcpServers?.jev, request.config.mcpServers.jev);
    if (provider === "opencode") assert.equal(result, request);
    else {
      assert.equal(result.config.cwd, "/workspace");
      const existing = injectPreflight(result, plugin, "/other");
      assert.equal(existing, result);
    }
  }
});

test("stdio protocol lists tool, binds workspace, rejects overrides and reports unavailable workspace", async (t) => {
  const root = await fixture(t, {
    ".paseo/preflight.json": {
      version: 1,
      runtimes: [{ executable: "node", major: 999, repair: "x" }],
      dependencies: [],
      ports: [],
      health: [],
    },
  });
  for (const workspace of [root, path.join(root, "unavailable")]) {
    const client = new Client({ name: "preflight-test", version: "1" });
    const injected = injectPreflight(
      { config: { provider: "codex", cwd: workspace } },
      plugin,
      workspace,
    ).config.mcpServers!.workspace_preflight;
    assert.equal(injected.type, "stdio");
    if (injected.type !== "stdio") throw new Error();
    const transport = new StdioClientTransport({
      command: injected.command,
      args: injected.args,
      env: injected.env,
      stderr: "pipe",
    });
    try {
      await client.connect(transport);
      const listed = await client.listTools();
      assert.deepEqual(
        listed.tools.map((tool) => tool.name),
        ["workspace_preflight"],
      );
      assert.equal(listed.tools[0].annotations?.readOnlyHint, true);
      const invalid = await client.callTool({
        name: "workspace_preflight",
        arguments: { directory: "/tmp" },
      });
      assert.equal(invalid.isError, true);
      const response = await client.callTool({ name: "workspace_preflight", arguments: {} });
      assert.ok(!response.isError, JSON.stringify(response));
      const report = reportSchema.parse(response.structuredContent);
      if (workspace === root)
        assert.equal(report.checks.find((c) => c.category === "runtimes")?.status, "blocker");
      else assert.equal(report.checks[0].label, "Workspace unavailable");
    } finally {
      await client.close();
      await transport.close();
    }
  }
});
