import {
  type PluginHostProps,
  type PluginWorkspacePanelProps,
  useRpc,
  useWorkspace,
} from "@getpaseo/plugin/client";
import { copyText, ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { attachmentKey, readBoardRpc, type Task } from "../shared/board";
import { dashboardSummary, groupOrder, groupTasks, taskTitle, type TaskGroup } from "./dashboard";

const groupLabels: Record<TaskGroup, string> = {
  active: "Active",
  blocked: "Blocked",
  todo: "Todo",
  done: "Done",
  unknown: "Unknown",
};

export function WatchtowerPanel(props: PluginWorkspacePanelProps) {
  const projectName = useWorkspace(props.workspaceId, (workspace) => workspace.projectDisplayName);
  return <WatchtowerBoard {...props} projectName={projectName} />;
}

// useWorkspace throws outside workspace panels, so the sidebar page passes projectName itself.
export function WatchtowerBoard({
  workspaceId,
  projectName,
  host,
  theme,
  layout,
}: PluginHostProps & { workspaceId: string; projectName: string | null }) {
  const read = useRpc(readBoardRpc);
  const board = useQuery({
    queryKey: ["watchtower-board", host.id, workspaceId],
    queryFn: () => read({ workspaceId }),
    retry: false,
  });
  const [selection, setSelection] = useState<{ workspaceId: string; taskId: string } | null>(null);
  const [collapsed, setCollapsed] = useState<Partial<Record<TaskGroup, boolean>>>({});
  const [panelWidth, setPanelWidth] = useState<number | null>(null);
  const [notice, setNotice] = useState("");
  const dense = layout.compact || (panelWidth !== null && panelWidth < 520);
  const selected =
    selection?.workspaceId === workspaceId
      ? board.data?.tasks.find((task) => task.id === selection.taskId)
      : null;
  const tasks = board.data?.tasks ?? [];
  const summary = useMemo(() => dashboardSummary(tasks), [tasks]);
  const groups = useMemo(() => groupTasks(tasks), [tasks]);
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
      group: { borderTopWidth: 1, borderTopColor: theme.colors.border },
      groupHeader: {
        minHeight: 38,
        paddingVertical: 9,
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 7,
      },
      groupChevron: { color: theme.colors.foregroundMuted, width: 12, fontSize: 12 },
      groupTitle: {
        flex: 1,
        color: theme.colors.foreground,
        fontSize: 13,
        fontWeight: "600" as const,
      },
      groupCount: { color: theme.colors.foregroundMuted, fontSize: 12 },
      task: {
        paddingVertical: 9,
        paddingHorizontal: dense ? 8 : 10,
        borderRadius: 6,
        gap: 8,
      },
      selectedTask: { backgroundColor: theme.colors.surface1 },
      taskRow: {
        flexDirection: "row" as const,
        alignItems: "flex-start" as const,
        gap: 8,
      },
      taskId: { color: theme.colors.foreground, fontSize: 12, fontWeight: "600" as const },
      taskName: {
        flex: 1,
        minWidth: 0,
        color: theme.colors.foreground,
        fontSize: 13,
        lineHeight: 18,
      },
      badge: {
        flexShrink: 0,
        paddingHorizontal: 6,
        paddingVertical: 2,
        borderWidth: 1,
        borderRadius: 5,
      },
      badgeText: {
        fontSize: 10,
        fontWeight: "600" as const,
        textTransform: "uppercase" as const,
      },
      details: {
        marginTop: 2,
        paddingTop: 10,
        borderTopWidth: 1,
        borderTopColor: theme.colors.border,
        gap: 8,
      },
      detailLabel: {
        color: theme.colors.foregroundMuted,
        fontSize: 10,
        fontWeight: "600" as const,
        textTransform: "uppercase" as const,
      },
      blocker: { color: theme.colors.statusDanger, fontSize: 13, lineHeight: 19 },
      error: { color: theme.colors.statusDanger, fontSize: 13, lineHeight: 19 },
      key: { color: theme.colors.foregroundMuted, fontSize: 11 },
      copyButton: {
        alignSelf: "flex-start" as const,
        paddingHorizontal: 10,
        paddingVertical: 7,
        borderRadius: 6,
        backgroundColor: theme.colors.accent,
      },
      copyButtonText: {
        color: theme.colors.accentForeground,
        fontSize: 12,
        fontWeight: "600" as const,
      },
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

  function badgeColor(status: string): string {
    if (status === "DONE") return theme.colors.statusSuccess;
    if (status === "BLOCKED") return theme.colors.statusDanger;
    if (status === "IN PROGRESS") return theme.colors.statusWarning;
    if (status === "TODO") return theme.colors.statusWarning;
    return theme.colors.statusDanger;
  }

  async function copySearch(task: Task) {
    try {
      await copyText(attachmentKey(workspaceId, task.id));
      setNotice(
        "Search key copied. Open composer + → Watchtower task, paste the key, then select the task.",
      );
    } catch {
      setNotice("Could not copy. Use the search key below in composer + → Watchtower task.");
    }
  }

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
          onPress={() => {
            setNotice("");
            void board.refetch();
          }}
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

          {visibleGroups.map((group) => {
            const isCollapsed = collapsed[group] === true;
            return (
              <View key={group} style={styles.group}>
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel={`${groupLabels[group]} tasks, ${groups[group].length}`}
                  accessibilityState={{ expanded: !isCollapsed }}
                  style={styles.groupHeader}
                  onPress={() => setCollapsed((current) => ({ ...current, [group]: !isCollapsed }))}
                >
                  <Text style={styles.groupChevron}>{isCollapsed ? "›" : "⌄"}</Text>
                  <Text style={styles.groupTitle}>{groupLabels[group]}</Text>
                  <Text style={styles.groupCount}>{groups[group].length}</Text>
                </Pressable>
                {!isCollapsed
                  ? groups[group].map((task) => {
                      const isSelected = selected?.id === task.id;
                      const color = badgeColor(task.status);
                      return (
                        <View
                          key={task.id}
                          style={[styles.task, isSelected ? styles.selectedTask : null]}
                        >
                          <Pressable
                            accessibilityRole="button"
                            accessibilityLabel={`${task.id}, ${taskTitle(task)}, ${groupLabels[group]}`}
                            accessibilityHint="Shows task details and attachment search"
                            accessibilityState={{ selected: isSelected, expanded: isSelected }}
                            style={styles.taskRow}
                            onPress={() => {
                              setSelection(isSelected ? null : { workspaceId, taskId: task.id });
                              setNotice("");
                            }}
                          >
                            <Text style={styles.taskId}>{task.id}</Text>
                            <Text
                              ellipsizeMode="tail"
                              numberOfLines={isSelected ? undefined : 1}
                              style={styles.taskName}
                            >
                              {taskTitle(task)}
                            </Text>
                            <View style={[styles.badge, { borderColor: color }]}>
                              <Text style={[styles.badgeText, { color }]}>
                                {groupLabels[group]}
                              </Text>
                            </View>
                          </Pressable>

                          {isSelected ? (
                            <View style={styles.details}>
                              <View>
                                <Text style={styles.detailLabel}>Brief</Text>
                                <Text selectable style={styles.text}>
                                  {task.brief ?? "Brief unavailable."}
                                </Text>
                              </View>
                              <View>
                                <Text style={styles.detailLabel}>Dependencies</Text>
                                <Text selectable style={styles.text}>
                                  {task.deps || "-"}
                                </Text>
                              </View>
                              {task.blocker ? (
                                <View>
                                  <Text style={styles.detailLabel}>Blocker</Text>
                                  <Text selectable style={styles.blocker}>
                                    {task.blocker}
                                  </Text>
                                </View>
                              ) : null}
                              {task.error ? (
                                <View>
                                  <Text style={styles.detailLabel}>Error</Text>
                                  <Text accessibilityRole="alert" selectable style={styles.error}>
                                    {task.error}
                                  </Text>
                                </View>
                              ) : null}
                              {task.brief && !task.error ? (
                                <>
                                  <Text selectable style={styles.key}>
                                    {attachmentKey(workspaceId, task.id)}
                                  </Text>
                                  <Pressable
                                    accessibilityRole="button"
                                    accessibilityLabel={`Copy attachment search for ${task.id}`}
                                    style={styles.copyButton}
                                    onPress={() => void copySearch(task)}
                                  >
                                    <Text style={styles.copyButtonText}>
                                      Copy attachment search
                                    </Text>
                                  </Pressable>
                                </>
                              ) : null}
                              {notice ? (
                                <Text accessibilityLiveRegion="polite" style={styles.muted}>
                                  {notice}
                                </Text>
                              ) : null}
                            </View>
                          ) : null}
                        </View>
                      );
                    })
                  : null}
              </View>
            );
          })}

          {tasks.length ? (
            <Text style={styles.muted}>
              Select a task for its brief. Attach through composer + → Watchtower task; nothing is
              sent automatically.
            </Text>
          ) : null}
        </>
      ) : null}
    </ScrollView>
  );
}
