import { tokenizeCode, type SyntaxTokenData } from "@getpaseo/plugin/client/ui";
import type { Hunk } from "../shared/diff";

// Highlights the old and new documents whole, so multi-line strings and comments keep their color.
export function highlightHunks(hunks: Hunk[], filePath: string): (SyntaxTokenData[] | null)[][] {
  const oldCode: string[] = [];
  const newCode: string[] = [];
  const positions = hunks.map((hunk) =>
    hunk.rows.map((row) => {
      if (row.kind === "remove") {
        oldCode.push(row.text);
        return { side: "old" as const, index: oldCode.length - 1 };
      }
      if (row.kind === "context") oldCode.push(row.text);
      newCode.push(row.text);
      return { side: "new" as const, index: newCode.length - 1 };
    }),
  );
  const oldTokens = oldCode.length > 0 ? tokenizeCode(oldCode.join("\n"), filePath) : null;
  const newTokens = newCode.length > 0 ? tokenizeCode(newCode.join("\n"), filePath) : null;
  return positions.map((rows) =>
    rows.map(({ side, index }) => (side === "old" ? oldTokens : newTokens)?.[index] ?? null),
  );
}
