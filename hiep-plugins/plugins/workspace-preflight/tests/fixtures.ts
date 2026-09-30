import { mkdtemp, mkdir, writeFile, readdir, unlink, rmdir } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import type { TestContext } from "node:test";

export async function clean(root: string): Promise<void> {
  for (const entry of await readdir(root, { withFileTypes: true })) {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) await clean(target);
    else await unlink(target);
  }
  await rmdir(root);
}
export async function fixture(t: Pick<TestContext, "after">, files: Record<string, unknown> = {}) {
  const root = await mkdtemp(path.join(os.tmpdir(), "preflight-discovery-"));
  t.after(() => clean(root));
  for (const [name, value] of Object.entries(files)) {
    await mkdir(path.dirname(path.join(root, name)), { recursive: true });
    await writeFile(
      path.join(root, name),
      typeof value === "string" ? value : JSON.stringify(value),
    );
  }
  return root;
}
