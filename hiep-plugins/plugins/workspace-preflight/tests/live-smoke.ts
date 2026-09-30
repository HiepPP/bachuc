import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { injectPreflight } from "../server/injection";
import { evaluateHandoff } from "../shared/evaluation";
import { liveCases, expectedChoices } from "./live-cases";
import { clean } from "./fixtures";

const jevDirectory = process.argv[2];
if (!jevDirectory || !path.isAbsolute(jevDirectory))
  throw new Error("Pass the absolute directory of the separately installed jev-evaluator plugin.");
const plugin = fileURLToPath(new URL("..", import.meta.url));
await mkdir(path.join(plugin, "artifacts"), { recursive: true });
const artifact = await mkdtemp(path.join(plugin, "artifacts", "jev-preflight-"));
const root = await mkdtemp(path.join(tmpdir(), "preflight-live-"));
const preflight = new Client({ name: "preflight-live", version: "1" });
const jev = new Client({ name: "preflight-jev-live", version: "1" });
const injected = injectPreflight({ config: { provider: "codex", cwd: root } }, plugin, root).config
  .mcpServers!.workspace_preflight;
if (injected.type !== "stdio") throw new Error("Expected stdio");
const preflightTransport = new StdioClientTransport({
  command: injected.command,
  args: injected.args,
  env: injected.env,
});
const jevTransport = new StdioClientTransport({
  command: process.execPath,
  args: [path.join(jevDirectory, "server/mcp.mjs")],
  env: {
    ELECTRON_RUN_AS_NODE: "1",
    PASEO_JEV_CONFIG_PATH:
      process.env.PASEO_JEV_CONFIG_PATH ?? path.join(homedir(), ".paseo/config.json"),
  },
});
const save = (name: string, value: unknown) =>
  writeFile(path.join(artifact, name), JSON.stringify(value, null, 2));
try {
  const cases = liveCases();
  await save("labels.json", expectedChoices);
  await save("inputs.json", cases);
  await preflight.connect(preflightTransport);
  await jev.connect(jevTransport);
  await save("tools.json", { preflight: await preflight.listTools(), jev: await jev.listTools() });
  await save(
    "workspace-measurements.json",
    await preflight.callTool({ name: "workspace_preflight", arguments: {} }),
  );
  for (const scenario of cases) {
    let raw: unknown;
    const outcome = await evaluateHandoff(
      scenario.input.state.task,
      scenario.input.state.evidence,
      async (input) => {
        const result = await jev.callTool({ name: "jev_evaluate", arguments: input }, undefined, {
          timeout: 46000,
        });
        raw = result;
        return result.isError ? result : result.structuredContent;
      },
    );
    await save(`${scenario.id}.json`, { input: scenario.input, raw, outcome });
    if (
      !outcome.evaluation.available ||
      outcome.evaluation.choice !== expectedChoices[scenario.id as keyof typeof expectedChoices]
    )
      process.exitCode = 1;
  }
} catch {
  await save("failure.json", {
    error: "MCP setup or protocol failure; no retries were attempted.",
  });
  process.exitCode = 1;
} finally {
  await preflight.close();
  await preflightTransport.close();
  await jev.close();
  await jevTransport.close();
  await clean(root);
  console.log(`Synthetic acceptance artifacts: ${artifact}`);
}
