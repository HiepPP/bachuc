import { mkdirSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { fileURLToPath } from "node:url";
import {
  POLICY,
  tier0,
} from "../../../../../hiep-plugins/plugins/jev-permission-gate/server/policy.ts";
import { isSecret, sample } from "./lib.ts";

const SEED = 20261023;
const MAX_LENGTH = 2000;
const NULL_COUNT = 120;
const DECIDED_COUNT = 30;

const repoRoot = fileURLToPath(new URL("../../../../../", import.meta.url));
const dataDir = path.join(repoRoot, ".dev", "decider-probe");
const dbPath = process.env.PROBE_DB ?? path.join(homedir(), ".bachuc", "timeline-cache.sqlite");

interface Candidate {
  command: string;
  agentId: string;
  ts: string;
  decision: string | null;
}

const db = new DatabaseSync(dbPath, { readOnly: true });
const rows = db
  .prepare(
    `SELECT agent_id, timestamp, json_extract(item, '$.detail.command') AS command
     FROM timeline_rows
     WHERE item_type = 'tool_call' AND json_extract(item, '$.detail.type') = 'shell'
     ORDER BY timestamp, agent_id, seq`,
  )
  .all() as { agent_id: string; timestamp: string; command: unknown }[];
db.close();

const first = new Map<string, Candidate>();
for (const row of rows) {
  if (typeof row.command !== "string" || first.has(row.command)) continue;
  first.set(row.command, {
    command: row.command,
    agentId: row.agent_id,
    ts: row.timestamp,
    decision: null,
  });
}

const pool = [...first.values()]
  .filter((c) => c.command.length <= MAX_LENGTH && !isSecret(c.command))
  .sort((a, b) => Number(a.command > b.command) - Number(a.command < b.command));
for (const c of pool) c.decision = tier0(c.command)?.decision ?? null;

const nullPool = pool.filter((c) => c.decision === null);
const decidedPool = pool.filter((c) => c.decision !== null);
if (nullPool.length < NULL_COUNT)
  console.warn(`warning: tier0-null pool has ${nullPool.length} commands, wanted ${NULL_COUNT}`);
if (decidedPool.length < DECIDED_COUNT)
  console.warn(`warning: decided pool has ${decidedPool.length} commands, wanted ${DECIDED_COUNT}`);

const picked = [...sample(nullPool, NULL_COUNT, SEED), ...sample(decidedPool, DECIDED_COUNT, SEED)];

mkdirSync(dataDir, { recursive: true });
const events = picked.map((c, i) => ({
  id: `e${String(i + 1).padStart(3, "0")}`,
  uc: "uc2",
  state: { policy: POLICY, command: c.command, tool: "Bash", cwd_relative: "." },
  meta: { tier0: c.decision, agent_id: c.agentId, ts: c.ts },
}));
const labels = events.map((e) => ({
  id: e.id,
  uc: "uc2",
  label: e.meta.tier0,
  read_only: null,
}));
const lines = (items: unknown[]) => items.map((item) => `${JSON.stringify(item)}\n`).join("");
writeFileSync(path.join(dataDir, "events.jsonl"), lines(events));
writeFileSync(path.join(dataDir, "labels.draft.jsonl"), lines(labels));
console.log(`wrote ${events.length} events to ${dataDir}`);
