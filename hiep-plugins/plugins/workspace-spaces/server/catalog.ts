import type { PaseoApi } from "@getpaseo/client";
export async function loadCatalog(paseo: Pick<PaseoApi, "projects" | "workspaces">) {
  const { projects } = await paseo.projects.list();
  const rows = new Map(
    projects.map((p) => [
      p.projectId,
      {
        id: p.projectId,
        name: p.projectDisplayName,
        path: p.projectRootPath,
        viewKey: p.projectKey,
        workspaces: [] as { id: string; name: string }[],
      },
    ]),
  );
  const cursors = new Set<string>();
  let cursor: string | undefined;
  do {
    const page = await paseo.workspaces.list({
      page: { limit: 200, ...(cursor ? { cursor } : {}) },
    });
    for (const workspace of page.entries) {
      const row = rows.get(workspace.projectId);
      if (row && !row.workspaces.some((w) => w.id === workspace.id))
        row.workspaces.push({ id: workspace.id, name: workspace.name });
    }
    if (!page.pageInfo.hasMore) break;
    cursor = page.pageInfo.nextCursor ?? undefined;
    if (!cursor || cursors.has(cursor))
      throw new Error("Workspace list changed. Refresh to retry.");
    cursors.add(cursor);
  } while (cursor);
  return { projects: [...rows.values()].sort((a, b) => a.name.localeCompare(b.name)) };
}
