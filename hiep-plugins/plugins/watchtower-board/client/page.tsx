import { type PluginSurfaceProps, usePaseo } from "@getpaseo/plugin/client";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Text, View } from "react-native";
import { WatchtowerBoard } from "./board";
import { WatchtowerOverview } from "./overview";

// Surfaces have no workspace context, so the opener names the workspace before it opens the page.
let lastWorkspaceId: string | null = null;

/** Picks the workspace the page shows when it next mounts. */
export function preselectWorkspace(workspaceId: string) {
  lastWorkspaceId = workspaceId;
}

// The page shows one workspace's board only: the one its opener named, or, when no opener named
// one (the mobile sidebar), the most recently active workspace. A named workspace never falls
// back to another one, because the overview's actions send prompts to that workspace's agent.
export function WatchtowerPage(props: PluginSurfaceProps) {
  const { host, theme, layout } = props;
  const paseo = usePaseo();
  const named = lastWorkspaceId;
  const workspaces = useQuery({
    queryKey: ["watchtower-board", host.id, "workspaces"],
    queryFn: async () =>
      (await paseo.workspaces.list({ sort: [{ key: "activity_at", direction: "desc" }] })).entries,
  });
  const entries = workspaces.data ?? [];
  const listed = entries.find((entry) => entry.id === named) ?? null;
  // The list can lag a new workspace or hold only its first page, so ask for the named one by ID.
  const lookup = useQuery({
    queryKey: ["watchtower-board", host.id, "workspace", named],
    queryFn: async () => (named ? await paseo.workspaces.ref(named).refresh() : null),
    enabled: named !== null && !listed && !workspaces.isPending,
    retry: false,
  });
  const workspace = named === null ? (entries[0] ?? null) : (listed ?? lookup.data ?? null);
  const styles = useMemo(
    () => ({
      screen: { flex: 1, backgroundColor: theme.colors.surface0 },
      message: { padding: layout.compact ? 16 : 24, color: theme.colors.foregroundMuted },
    }),
    [theme, layout.compact],
  );

  if (!workspace) {
    const looking = workspaces.isPending || lookup.isPending;
    const message =
      named !== null
        ? looking
          ? "Loading workspace…"
          : lookup.isError
            ? "Could not load the workspace."
            : "Workspace not found."
        : workspaces.isError
          ? "Could not load workspaces."
          : workspaces.isPending
            ? "Loading workspaces…"
            : "No workspaces on this host.";
    return (
      <View style={styles.screen}>
        <Text style={styles.message}>{message}</Text>
      </View>
    );
  }

  return (
    <View style={styles.screen}>
      {/* Wide layouts have room for the process overview; compact ones keep the board. */}
      {layout.compact ? (
        <WatchtowerBoard
          {...props}
          key={workspace.id}
          source={{ workspaceId: workspace.id }}
          projectName={workspace.projectDisplayName}
        />
      ) : (
        <WatchtowerOverview
          {...props}
          key={workspace.id}
          workspaceId={workspace.id}
          projectName={workspace.projectDisplayName}
        />
      )}
    </View>
  );
}
