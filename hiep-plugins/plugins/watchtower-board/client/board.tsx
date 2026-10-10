import {
  type PluginHostProps,
  type PluginNewWorkspacePanelProps,
  useRpc,
} from "@getpaseo/plugin/client";
import { ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { Pressable, Text, View } from "react-native";
import { readProjectBoardRpc } from "../shared/board";
import { ProgressCard, TasksCard, WaitingCard } from "./board-rail";

// The new workspace screen has no workspace yet, so it has no run, agent, or overview. It shows
// the rail cards for the project's root.
export function WatchtowerProjectPanel(props: PluginNewWorkspacePanelProps) {
  const folder = props.cwd.split(/[\\/]/).filter(Boolean).pop() ?? null;
  return <ProjectBoard {...props} projectId={props.projectId} projectName={folder} />;
}

function ProjectBoard({
  projectId,
  projectName,
  host,
  theme,
}: PluginHostProps & { projectId: string; projectName: string | null }) {
  const { colors } = theme;
  const read = useRpc(readProjectBoardRpc);
  const sourceKey = `project:${projectId}`;
  const board = useQuery({
    queryKey: ["watchtower-board", host.id, sourceKey],
    queryFn: () => read({ projectId }),
    retry: false,
  });

  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: colors.surface0 }}
      contentContainerStyle={{ padding: 12, gap: 12 }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: 12 }}>
        <View style={{ flex: 1, minWidth: 0 }}>
          <Text
            numberOfLines={2}
            style={{ fontSize: 17, fontWeight: "600", color: colors.foreground }}
          >
            {board.data?.title ?? "Watchtower"}
          </Text>
          <Text numberOfLines={1} style={{ fontSize: 12, color: colors.foregroundMuted }}>
            {projectName ?? "Project"} · Read-only
          </Text>
        </View>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Refresh Watchtower board"
          accessibilityState={{ disabled: board.isFetching, busy: board.isFetching }}
          disabled={board.isFetching}
          onPress={() => void board.refetch()}
          style={{
            paddingHorizontal: 12,
            paddingVertical: 8,
            borderWidth: 1,
            borderColor: colors.border,
            borderRadius: 6,
            backgroundColor: colors.surface2,
          }}
        >
          <Text style={{ fontSize: 13, fontWeight: "600", color: colors.foreground }}>
            {board.isFetching ? "Loading…" : "Refresh"}
          </Text>
        </Pressable>
      </View>
      {board.data ? <ProgressCard tasks={board.data.tasks} theme={theme} /> : null}
      {board.data?.warnings.map((warning) => (
        <Text key={warning} style={{ fontSize: 12, color: colors.statusDanger }}>
          {warning}
        </Text>
      ))}
      {board.data ? (
        <WaitingCard
          questions={board.data.questions}
          proposedAdrs={board.data.proposedAdrs}
          theme={theme}
        />
      ) : null}
      <TasksCard
        board={board.data}
        error={board.error}
        workspaceId={null}
        sourceKey={sourceKey}
        theme={theme}
      />
    </ScrollView>
  );
}
