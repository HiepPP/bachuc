import MarkdownIt from "markdown-it";
import { setImmediate } from "node:timers/promises";
import type { AgentTimelineRow } from "../agent-timeline-store-types.js";
import { projectTimelineRows } from "../timeline-projection.js";

const markdown = new MarkdownIt({ html: false });
const PAGE_SIZE = 200;
interface SearchLocation {
  seq: number;
  role: "user" | "assistant";
  count: number;
}
export interface SearchResult {
  locations: SearchLocation[];
  nextCursor: number | null;
}
interface SearchInput {
  rows: readonly AgentTimelineRow[];
  query: string;
  cursor?: number;
}
export interface SearchCandidate {
  /** Last source seq of the projected message. */
  seq: number;
  role: "user" | "assistant";
  text: string;
}
interface CandidateSearchInput {
  candidates: Iterable<SearchCandidate> | AsyncIterable<SearchCandidate>;
  query: string;
  cursor?: number;
  foldDiacritics?: boolean;
}

// Discovery is deliberately approximate. Only the client can verify displayed text, so
// `count` estimates rendered occurrences and the client replaces it with the rendered
// count once the message is on screen. Blocks are counted separately, and the Markdown
// source is never searched whole, because a match the client cannot highlight — across
// a block boundary, or on syntax the reader never sees — still inflates the total until
// navigation reaches that message and drops it.
function searchableBlocks(text: string): string[] {
  return markdown
    .parse(text, {})
    .map((token) => {
      if (token.type === "inline") {
        return (token.children ?? [])
          .map((child) => {
            if (child.type === "text" || child.type === "code_inline") return child.content;
            if (child.type === "softbreak" || child.type === "hardbreak") return "\n";
            return "";
          })
          .join("");
      }
      if (token.type === "fence" || token.type === "code_block") return token.content;
      return "";
    })
    .filter(Boolean);
}

function searchPattern(query: string): RegExp {
  const source = query
    .trim()
    .split(/\s+/)
    .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("\\s+");
  return new RegExp(source, "giu");
}

function countMatches(text: string, pattern: RegExp): number {
  let count = 0;
  for (const _ of text.matchAll(pattern)) count += 1;
  return count;
}

function stripDiacritics(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").replace(/đ/g, "d").replace(/Đ/g, "D");
}

function* projectedMessages(rows: readonly AgentTimelineRow[]): Generator<SearchCandidate> {
  for (const entry of projectTimelineRows({ rows, mode: "projected" })) {
    const item = entry.item;
    if (item.type !== "user_message" && item.type !== "assistant_message") continue;
    yield {
      seq: entry.seqEnd,
      role: item.type === "user_message" ? "user" : "assistant",
      text: item.text,
    };
  }
}

export function searchTimeline({ rows, query, cursor = 0 }: SearchInput): Promise<SearchResult> {
  return searchCandidates({ candidates: projectedMessages(rows), query, cursor });
}

/**
 * Counts matches in candidate messages, in seq order. With `foldDiacritics`, a query typed
 * without diacritics also matches text with them, as the FTS5 index does; a query with
 * diacritics keeps exact matching.
 */
export async function searchCandidates({
  candidates,
  query,
  cursor = 0,
  foldDiacritics = false,
}: CandidateSearchInput): Promise<SearchResult> {
  const locations: SearchLocation[] = [];
  if (!query.trim()) return { locations, nextCursor: null };
  const folds = foldDiacritics && stripDiacritics(query) === query.normalize("NFC");
  const pattern = searchPattern(folds ? stripDiacritics(query) : query);
  let lastYield = performance.now();
  for await (const candidate of candidates) {
    if (candidate.seq <= cursor) continue;
    const text = candidate.text.replace(/\r/g, "");
    const raw = folds ? stripDiacritics(text) : text;
    const blocks = candidate.role === "assistant" ? searchableBlocks(raw) : [raw];
    const count = blocks.reduce((sum, block) => sum + countMatches(block, pattern), 0);
    if (count) {
      if (locations.length === PAGE_SIZE) {
        return { locations, nextCursor: locations[PAGE_SIZE - 1]!.seq };
      }
      locations.push({ seq: candidate.seq, role: candidate.role, count });
    }
    if (performance.now() - lastYield >= 8) {
      await setImmediate();
      lastYield = performance.now();
    }
  }
  return { locations, nextCursor: null };
}
