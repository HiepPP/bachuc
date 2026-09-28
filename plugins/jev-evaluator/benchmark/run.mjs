import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { createHash } from "node:crypto";
import { appendFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const resumeDirectory = process.argv[2];
const spacingMs = resumeDirectory ? 65_000 : 25_000;
const datasetBytes = await readFile(
  resumeDirectory
    ? path.join(resumeDirectory, "cases.json")
    : new URL("./cases.json", import.meta.url),
);
const dataset = JSON.parse(datasetBytes.toString("utf8"));
const datasetSha256 = createHash("sha256").update(datasetBytes).digest("hex");
/** @type {Record<string, string>} */
const runtimeHashes = {};
for (const name of ["../server/mcp.mjs", "../server/evaluate.mjs", "../package-lock.json"]) {
  runtimeHashes[name] = createHash("sha256")
    .update(await readFile(new URL(name, import.meta.url)))
    .digest("hex");
}
const startedAt = new Date().toISOString();
const artifacts = fileURLToPath(new URL("../artifacts/", import.meta.url));
const output =
  resumeDirectory || path.join(artifacts, `jev-benchmark-${startedAt.replaceAll(/[:.]/g, "-")}`);
/** @type {import("./score.mjs").BenchmarkRecord[]} */
const records = [];

// Gold labels remain on disk. Only state and questions enter the MCP request.
for (const batch of dataset.batches) {
  const ids = Object.keys(batch.questions).sort();
  if (JSON.stringify(ids) !== JSON.stringify(Object.keys(batch.expected).sort())) {
    throw new Error(`Question/label mismatch: ${batch.id}`);
  }
}
if (resumeDirectory) {
  const metadata = JSON.parse(await readFile(path.join(output, "metadata.json"), "utf8"));
  if (
    metadata.datasetSha256 !== datasetSha256 ||
    JSON.stringify(metadata.runtimeHashes) !== JSON.stringify(runtimeHashes)
  ) {
    throw new Error("Cannot resume with changed dataset or evaluator runtime.");
  }
  const lines = await readFile(path.join(output, "attempts.jsonl"), "utf8");
  records.push(
    ...lines
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line)),
  );
  await appendFile(
    path.join(output, "resumptions.jsonl"),
    `${JSON.stringify({ startedAt, spacingMs, reason: "Resume unanswered batches after rate-limit cooldown; preserve all prior attempts." })}\n`,
  );
} else {
  await mkdir(artifacts, { recursive: true });
  await mkdir(output);
  await writeFile(path.join(output, "cases.json"), datasetBytes, { flag: "wx" });
  await writeFile(path.join(output, "attempts.jsonl"), "", { flag: "wx" });
  await writeFile(
    path.join(output, "metadata.json"),
    JSON.stringify(
      {
        startedAt,
        datasetSha256,
        runtimeHashes,
        model: "jev-latest",
        transport: "MCP stdio jev_evaluate",
        questionCount: dataset.batches.reduce(
          /** @param {number} total @param {{questions: Record<string, unknown>}} batch */
          (total, batch) => total + Object.keys(batch.questions).length,
          0,
        ),
        spacingMs,
        retry: "Only HTTP 429, once after 70 seconds; never retry a model answer.",
      },
      null,
      2,
    ),
    { flag: "wx" },
  );
}
console.log(`Evidence: ${output}`);

const client = new Client({ name: "jev-known-answer-benchmark", version: "0.1.0" });
const transport = new StdioClientTransport({
  command: process.execPath,
  args: [fileURLToPath(new URL("../server/mcp.mjs", import.meta.url))],
  env: {
    ELECTRON_RUN_AS_NODE: "1",
    PASEO_JEV_CONFIG_PATH:
      process.env.PASEO_JEV_CONFIG_PATH || path.join(homedir(), ".paseo", "config.json"),
  },
});
try {
  await client.connect(transport);
  for (const [index, batch] of dataset.batches.entries()) {
    const previous = records.filter((record) => record.batchId === batch.id);
    if (previous.some((record) => !record.isError)) continue;
    if (index > 0) {
      await delay(Math.min(spacingMs, 35_000));
      if (spacingMs > 35_000) await delay(spacingMs - 35_000);
    }
    const previousAttempt = Math.max(0, ...previous.map((record) => record.attempt));
    for (let retry = 0; retry < 2; retry++) {
      const attempt = previousAttempt + retry + 1;
      const start = performance.now();
      const response = await client.callTool({
        name: "jev_evaluate",
        arguments: { state: batch.state, questions: batch.questions },
      });
      const record = {
        batchId: batch.id,
        attempt,
        latencyMs: Math.round(performance.now() - start),
        result: response.structuredContent,
        isError: response.isError === true,
      };
      records.push(record);
      await appendFile(path.join(output, "attempts.jsonl"), `${JSON.stringify(record)}\n`);
      console.log(
        `${index + 1}/${dataset.batches.length} ${batch.id}: ${record.isError ? "ERROR" : "OK"} (${record.latencyMs}ms)`,
      );
      const content = response.structuredContent;
      const status =
        content !== null && typeof content === "object" && "error" in content
          ? content.error
          : undefined;
      if (
        retry === 0 &&
        record.isError &&
        status !== null &&
        typeof status === "object" &&
        "status" in status &&
        status.status === 429
      ) {
        console.log("429: retaining failure; waiting 70 seconds before the one allowed retry.");
        await delay(35_000);
        await delay(35_000);
        continue;
      }
      break;
    }
  }
  const { summarize, renderReport } = await import("./score.mjs");
  const summary = summarize(dataset, records);
  await writeFile(path.join(output, "summary.json"), JSON.stringify(summary, null, 2), {
    flag: "wx",
  });
  await writeFile(path.join(output, "report.md"), renderReport(summary), { flag: "wx" });
  console.log(`Report: ${path.join(output, "report.md")}`);
} finally {
  await client.close();
  await transport.close();
}
