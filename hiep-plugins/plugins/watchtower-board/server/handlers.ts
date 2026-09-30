import type { PaseoApi, PaseoWorkspace } from "@getpaseo/client";
import type { PluginAttachmentItem } from "@getpaseo/plugin";
import { pathToFileURL } from "node:url";
import path from "node:path";
import { attachmentKey } from "../shared/board";
import { readBoard } from "./board";

export async function loadWorkspaceBoard(workspaceId: string, paseo: PaseoApi) {
  const workspace = await paseo.workspaces.ref(workspaceId).refresh();
  if (!workspace) throw new Error("Workspace is unavailable on this host.");
  return readBoard(workspace.workspaceDirectory);
}

export async function searchTaskAttachments(
  query: string,
  paseo: PaseoApi,
): Promise<{ items: PluginAttachmentItem[] }> {
  const scoped = query.trim().match(/^workspace:(\S+)(?:\s+(.*))?$/);
  let workspaces: PaseoWorkspace[];
  let search = query.trim().toLowerCase();
  if (scoped) {
    const workspace = await paseo.workspaces.ref(scoped[1]).refresh();
    if (!workspace) throw new Error("Workspace is unavailable on this host.");
    workspaces = [workspace];
    search = (scoped[2] ?? "").toLowerCase();
  } else {
    const result = await paseo.workspaces.list({
      page: { limit: 30 },
      sort: [{ key: "activity_at", direction: "desc" }],
    });
    workspaces = result.entries;
  }
  const items: PluginAttachmentItem[] = [];
  for (const workspace of workspaces) {
    const board = await readBoard(workspace.workspaceDirectory);
    if (scoped && board.message && !board.tasks.length) throw new Error(board.message);
    for (const task of board.tasks) {
      const label = `${workspace.projectDisplayName} / ${workspace.name}`;
      const haystack =
        `${label} ${workspace.workspaceDirectory} ${task.title} ${task.status}`.toLowerCase();
      if (
        (scoped && /^task-\d+$/.test(search) && task.id.toLowerCase() !== search) ||
        task.error ||
        !task.brief ||
        !search.split(/\s+/).every((word) => haystack.includes(word))
      )
        continue;
      items.push({
        id: `${workspace.id}:${task.id}`,
        identifier: task.id,
        title: `${label} · ${task.title}`,
        subtitle: `${task.status} · ${workspace.workspaceDirectory}${
          scoped ? ` · ${attachmentKey(workspace.id, task.id)}` : ""
        }`,
        resourceType: "Watchtower task",
        url: pathToFileURL(path.join(workspace.workspaceDirectory, "watchtower", "NEXT.md")).href,
        text: [
          `# ${task.title}`,
          `Workspace: ${workspace.workspaceDirectory}`,
          `Status: ${task.status}`,
          `Dependencies: ${task.deps || "-"}`,
          `Spec: ${task.spec}`,
          ...(task.blocker ? [`Blocker: ${task.blocker}`] : []),
          "",
          "## Brief",
          task.brief,
        ].join("\n"),
      });
      if (items.length === 20) return { items };
    }
  }
  return { items };
}
