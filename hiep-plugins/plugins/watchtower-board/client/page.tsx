import { type PluginSurfaceProps, usePaseo } from "@getpaseo/plugin/client";
import { ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { WatchtowerBoard } from "./board";

// Surfaces have no workspace context, so the sidebar page picks one and reuses the board.
let lastWorkspaceId: string | null = null;

export function WatchtowerPage(props: PluginSurfaceProps) {
  const { host, theme, layout } = props;
  const paseo = usePaseo();
  const workspaces = useQuery({
    queryKey: ["watchtower-board", host.id, "workspaces"],
    queryFn: async () =>
      (await paseo.workspaces.list({ sort: [{ key: "activity_at", direction: "desc" }] })).entries,
  });
  const [picked, setPicked] = useState(lastWorkspaceId);
  const entries = workspaces.data ?? [];
  const workspace = entries.find((entry) => entry.id === picked) ?? entries[0] ?? null;
  const styles = useMemo(
    () => ({
      screen: { flex: 1, backgroundColor: theme.colors.surface0 },
      picker: {
        flexGrow: 0,
        borderBottomWidth: 1,
        borderBottomColor: theme.colors.border,
      },
      pickerContent: { padding: layout.compact ? 12 : 16, gap: 8 },
      chip: {
        paddingHorizontal: 12,
        paddingVertical: 6,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 999,
        backgroundColor: theme.colors.surface1,
      },
      chipSelected: { borderColor: theme.colors.accent, backgroundColor: theme.colors.surface2 },
      chipText: { color: theme.colors.foreground, fontSize: 13 },
      chipMuted: { color: theme.colors.foregroundMuted },
      message: { padding: layout.compact ? 16 : 24, color: theme.colors.foregroundMuted },
    }),
    [theme, layout.compact],
  );

  if (!workspace) {
    const message = workspaces.isError
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
      <ScrollView horizontal style={styles.picker} contentContainerStyle={styles.pickerContent}>
        {entries.map((entry) => {
          const selected = entry.id === workspace.id;
          return (
            <Pressable
              key={entry.id}
              accessibilityRole="button"
              accessibilityState={{ selected }}
              accessibilityLabel={`Show Watchtower for ${entry.projectDisplayName} ${entry.name}`}
              onPress={() => {
                lastWorkspaceId = entry.id;
                setPicked(entry.id);
              }}
              style={[styles.chip, selected && styles.chipSelected]}
            >
              <Text style={styles.chipText} numberOfLines={1}>
                {entry.projectDisplayName}
                <Text style={styles.chipMuted}> · {entry.name}</Text>
              </Text>
            </Pressable>
          );
        })}
      </ScrollView>
      <WatchtowerBoard
        {...props}
        key={workspace.id}
        workspaceId={workspace.id}
        projectName={workspace.projectDisplayName}
      />
    </View>
  );
}
