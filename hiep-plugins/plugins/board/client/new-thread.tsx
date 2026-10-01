import type { PluginClientContext } from "@getpaseo/plugin/client";
import type { BoardRun } from "../shared/board";

// Remove and Start New Thread opens the new workspace screen for the run's project.
export function installNewThreadNavigation(client: PluginClientContext) {
  return {
    open(run: BoardRun) {
      if (run.cwd)
        client.openNewWorkspace({ cwd: run.cwd, name: run.project, projectId: run.projectId });
    },
    cleanup: () => {},
  };
}
