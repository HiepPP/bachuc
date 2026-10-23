// Prints the gate's recent decisions with the command text. The ledger stores only command
// hashes, so this matches them against shell commands in the daemon's timeline cache.
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { formatDecision, selectDecisions } from "../server/decisions";
import { hashCommand, type LedgerEntry } from "../server/ledger";

const args = process.argv.slice(2);
const option = (name: string) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};
const home = path.resolve(
  (option("--home") ?? process.env.PASEO_HOME ?? "~/.bachuc").replace(/^~/, homedir()),
);
const limit = Number(option("--limit") ?? 30);

const ledgerFile = path.join(home, "plugin-data/jev-permission-gate/decisions.jsonl");
let entries: LedgerEntry[];
try {
  entries = readFileSync(ledgerFile, "utf8")
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
} catch {
  console.error(`No decisions yet: ${ledgerFile}`);
  process.exit(1);
}

const commands = new Map<string, string>();
const db = new DatabaseSync(path.join(home, "timeline-cache.sqlite"), { readOnly: true });
const rows = db
  .prepare(
    "select distinct json_extract(item, '$.detail.command') as command from timeline_rows " +
      "where item_type = 'tool_call' and json_extract(item, '$.detail.type') = 'shell'",
  )
  .all() as { command: string | null }[];
for (const { command } of rows) if (command) commands.set(hashCommand(command), command);
db.close();

const selected = selectDecisions(entries, {
  jevAllowedOnly: args.includes("--jev-allowed"),
  limit,
});
for (const entry of selected) console.log(formatDecision(entry, commands.get(entry.commandHash)));
console.log(`${selected.length} of ${entries.length} decisions shown from ${ledgerFile}`);
