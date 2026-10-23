import type { LedgerEntry } from "./ledger";

export interface DecisionFilter {
  jevAllowedOnly: boolean;
  limit: number;
}

/** Newest-last slice of the ledger, optionally only commands Jev allowed on its own. */
export function selectDecisions(entries: LedgerEntry[], filter: DecisionFilter): LedgerEntry[] {
  const kept = filter.jevAllowedOnly
    ? entries.filter((entry) => entry.source === "jev" && entry.decision === "allow")
    : entries;
  return kept.slice(-filter.limit);
}

export function formatDecision(entry: LedgerEntry, command: string | undefined): string {
  const text = command ?? "(command no longer in the timeline cache)";
  return `${entry.ts.slice(0, 19)}  ${entry.decision.padEnd(8)} ${entry.source.padEnd(11)} ${text.replace(/\s+/g, " ").slice(0, 160)}`;
}
