import {
  type PluginHostProps,
  type PluginNewWorkspacePanelProps,
  type PluginWorkspacePanelProps,
  useRpc,
  useWorkspace,
} from "@getpaseo/plugin/client";
import { ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { readProjectBoardRpc } from "../shared/board";
import { dashboardSummary, groupOrder, plural } from "./dashboard";
import { WatchtowerOverview } from "./overview";
import { groupLabels, TaskList } from "./task-list";

// Explorer shows the same view as the Watchtower page; it lays itself out for the panel width.
export function WatchtowerPanel(props: PluginWorkspacePanelProps) {
  const projectName = useWorkspace(props.workspaceId, (workspace) => workspace.projectDisplayName);
  return (
    <WatchtowerOverview {...props} workspaceId={props.workspaceId} projectName={projectName} />
  );
}

// The new workspace screen has no workspace yet, so it has no run, agent, or overview. It shows
// the tasks of the project's root.
export function WatchtowerProjectPanel(props: PluginNewWorkspacePanelProps) {
  const folder = props.cwd.split(/[\\/]/).filter(Boolean).pop() ?? null;
  return <ProjectBoard {...props} projectId={props.projectId} projectName={folder} />;
}

function ProjectBoard({
  projectId,
  projectName,
  host,
  theme,
  layout,
}: PluginHostProps & { projectId: string; projectName: string | null }) {
  const read = useRpc(readProjectBoardRpc);
  const sourceKey = `project:${projectId}`;
  const board = useQuery({
    queryKey: ["watchtower-board", host.id, sourceKey],
    queryFn: () => read({ projectId }),
    retry: false,
  });
  const [panelWidth, setPanelWidth] = useState<number | null>(null);
  const dense = layout.compact || (panelWidth !== null && panelWidth < 520);
  const tasks = board.data?.tasks ?? [];
  const questions = board.data?.questions ?? [];
  const proposedAdrs = board.data?.proposedAdrs ?? 0;
  const summary = useMemo(() => dashboardSummary(tasks), [tasks]);
  const visibleGroups = groupOrder.filter(
    (group) => group !== "unknown" || summary.counts.unknown > 0,
  );
  const styles = useMemo(
    () => ({
      screen: { flex: 1, backgroundColor: theme.colors.surface0 },
      content: { padding: dense ? 12 : 20, gap: dense ? 10 : 12 },
      header: { flexDirection: "row" as const, alignItems: "center" as const, gap: 12 },
      headingCopy: { flex: 1, minWidth: 0 },
      title: {
        color: theme.colors.foreground,
        fontSize: dense ? 17 : 19,
        fontWeight: "600" as const,
      },
      text: { color: theme.colors.foreground, fontSize: 14, lineHeight: 20 },
      muted: { color: theme.colors.foregroundMuted, fontSize: 12, lineHeight: 17 },
      card: {
        padding: dense ? 10 : 12,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 8,
        backgroundColor: theme.colors.surface1,
      },
      refresh: {
        flexShrink: 0,
        paddingHorizontal: 12,
        paddingVertical: 8,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 6,
        backgroundColor: theme.colors.surface2,
      },
      refreshText: { color: theme.colors.foreground, fontSize: 13, fontWeight: "600" as const },
      progressTop: {
        flexDirection: "row" as const,
        justifyContent: "space-between" as const,
        alignItems: "baseline" as const,
        gap: 12,
      },
      percentage: { color: theme.colors.foreground, fontSize: 24, fontWeight: "700" as const },
      progressTrack: {
        height: 6,
        marginTop: 8,
        borderRadius: 3,
        overflow: "hidden" as const,
        backgroundColor: theme.colors.surface2,
      },
      progressFill: { height: 6, borderRadius: 3, backgroundColor: theme.colors.statusSuccess },
      counts: { flexDirection: "row" as const, gap: 6 },
      count: {
        flex: 1,
        minWidth: 0,
        paddingHorizontal: 4,
        paddingVertical: 8,
        alignItems: "center" as const,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 6,
        backgroundColor: theme.colors.surface1,
      },
      countLabel: {
        color: theme.colors.foregroundMuted,
        fontSize: 10,
        textTransform: "uppercase" as const,
      },
      countValue: { color: theme.colors.foreground, fontSize: 15, fontWeight: "700" as const },
      section: { gap: 6 },
      taskId: { color: theme.colors.foreground, fontSize: 12, fontWeight: "600" as const },
      detailLabel: {
        color: theme.colors.foregroundMuted,
        fontSize: 10,
        fontWeight: "600" as const,
        textTransform: "uppercase" as const,
      },
      error: { color: theme.colors.statusDanger, fontSize: 13, lineHeight: 19 },
      stateCard: {
        padding: 12,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 8,
        backgroundColor: theme.colors.surface1,
      },
    }),
    [theme, dense],
  );

  return (
    <ScrollView
      style={styles.screen}
      contentContainerStyle={styles.content}
      onLayout={(event) => setPanelWidth(event.nativeEvent.layout.width)}
    >
      <View style={styles.header}>
        <View style={styles.headingCopy}>
          <Text numberOfLines={2} style={styles.title}>
            {board.data?.title ?? "Watchtower"}
          </Text>
          <Text numberOfLines={1} style={styles.muted}>
            {projectName ?? "Project"} · Read-only
          </Text>
        </View>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Refresh Watchtower board"
          accessibilityState={{ disabled: board.isFetching, busy: board.isFetching }}
          disabled={board.isFetching}
          style={styles.refresh}
          onPress={() => void board.refetch()}
        >
          <Text style={styles.refreshText}>{board.isFetching ? "Loading…" : "Refresh"}</Text>
        </Pressable>
      </View>

      {board.error ? (
        <View style={styles.stateCard}>
          <Text accessibilityRole="alert" style={styles.error}>
            Could not load board. {board.error.message}
          </Text>
        </View>
      ) : null}

      {!board.data && !board.error ? (
        <View style={styles.stateCard}>
          <Text accessibilityLiveRegion="polite" style={styles.muted}>
            Loading Watchtower board…
          </Text>
        </View>
      ) : null}

      {board.data ? (
        <>
          <View
            accessible
            accessibilityLabel={`${summary.percentage}% complete, ${summary.counts.done} of ${summary.total} done`}
            style={styles.card}
          >
            <View style={styles.progressTop}>
              <Text style={styles.percentage}>{summary.percentage}%</Text>
              <Text style={styles.muted}>
                {summary.counts.done} of {summary.total} done
              </Text>
            </View>
            <View style={styles.progressTrack}>
              <View style={[styles.progressFill, { width: `${summary.percentage}%` }]} />
            </View>
          </View>

          <View accessibilityRole="summary" style={styles.counts}>
            {visibleGroups.map((group) => (
              <View key={group} style={styles.count}>
                <Text style={styles.countValue}>{summary.counts[group]}</Text>
                <Text numberOfLines={1} style={styles.countLabel}>
                  {groupLabels[group]}
                </Text>
              </View>
            ))}
          </View>

          {board.data.message ? (
            <View style={styles.stateCard}>
              <Text accessibilityLiveRegion="polite" style={styles.text}>
                {board.data.message}
              </Text>
            </View>
          ) : null}

          {board.data.warnings.map((warning) => (
            <View key={warning} style={styles.stateCard}>
              <Text style={styles.error}>{warning}</Text>
            </View>
          ))}

          {questions.length || proposedAdrs ? (
            <View style={[styles.card, styles.section]}>
              <Text style={styles.detailLabel}>Waiting on the owner</Text>
              <Text style={styles.muted}>
                {plural(questions.length, "blocking question")} ·{" "}
                {plural(proposedAdrs, "proposed ADR")}
              </Text>
              {questions.map((question) => (
                <View key={question.id}>
                  <Text style={styles.taskId}>
                    {question.id} · blocks {question.blocks.join(", ")}
                  </Text>
                  <Text selectable style={styles.text}>
                    {question.question}
                  </Text>
                </View>
              ))}
            </View>
          ) : null}

          <TaskList
            tasks={tasks}
            questions={questions}
            workspaceId={null}
            sourceKey={sourceKey}
            dense={dense}
            theme={theme}
          />
        </>
      ) : null}
    </ScrollView>
  );
}
