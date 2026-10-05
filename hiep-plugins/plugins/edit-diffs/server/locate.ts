import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import type { RpcInput, RpcOutput } from "@getpaseo/plugin";
import type { Located } from "../shared/diff";
import type { locateRpc } from "../shared/contracts";

const MAX_FILE_BYTES = 10_000_000;

function lineAt(content: string, offset: number): number {
  let line = 1;
  for (let index = content.indexOf("\n"); index !== -1 && index < offset; ) {
    line++;
    index = content.indexOf("\n", index + 1);
  }
  return line;
}

async function readTarget(filePath: string, cwd: string | null): Promise<string | null> {
  const target = path.isAbsolute(filePath) ? filePath : cwd ? path.resolve(cwd, filePath) : null;
  if (!target) return null;
  try {
    if ((await stat(target)).size > MAX_FILE_BYTES) return null;
    return await readFile(target, "utf8");
  } catch {
    return null;
  }
}

// The new text is in the file once the edit applied; the old text while it awaits permission.
export async function locate({
  filePath,
  cwd,
  hunks,
}: RpcInput<typeof locateRpc>): Promise<RpcOutput<typeof locateRpc>> {
  const content = await readTarget(filePath, cwd);
  if (content === null) return { located: hunks.map(() => null) };
  let from = 0;
  const located = hunks.map((hunk): Located | null => {
    for (const side of ["new", "old"] as const) {
      const text = side === "new" ? hunk.newText : hunk.oldText;
      if (!text) continue;
      let at = content.indexOf(text, from);
      if (at === -1) at = content.indexOf(text);
      if (at === -1) continue;
      from = at + text.length;
      return { line: lineAt(content, at), side };
    }
    return null;
  });
  return { located };
}
