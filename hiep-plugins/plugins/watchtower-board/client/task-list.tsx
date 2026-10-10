import type { PluginTheme } from "@getpaseo/plugin";
import { copyText } from "@getpaseo/plugin/client/react-native";
import { useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { attachmentKey, type Question, type Task } from "../shared/board";
import {
  dashboardSummary,
  groupOrder,
  groupTasks,
  questionsFor,
  taskTitle,
  type TaskGroup,
} from "./dashboard";

export const groupLabels: Record<TaskGroup, string> = {
  active: "Active",
  blocked: "Blocked",
  todo: "Todo",
  done: "Done",
  unknown: "Unknown",
};

interface TaskListProps {
  tasks: readonly Task[];
  questions: readonly Question[];
  // Attachment search keys name a workspace, so a project board has none.
  workspaceId: string | null;
  // Selection resets when the board source changes.
  sourceKey: string;
  dense: boolean;
  theme: PluginTheme;
  // Groups that start collapsed.
  collapsedGroups?: readonly TaskGroup[];
}

// Tasks grouped by status. Selecting a task shows its brief, deps, class, blockers, and attachment search.
export function TaskList({
  tasks,
  questions,
  workspaceId,
  sourceKey,
  dense,
  theme,
  collapsedGroups = [],
}: TaskListProps) {
  const [selection, setSelection] = useState<{ sourceKey: string; taskId: string } | null>(null);
  const [collapsed, setCollapsed] = useState<Partial<Record<TaskGroup, boolean>>>(() =>
    Object.fromEntries(collapsedGroups.map((group) => [group, true])),
  );
  const [notice, setNotice] = useState("");
  const selected =
    selection?.sourceKey === sourceKey ? tasks.find((task) => task.id === selection.taskId) : null;
  const summary = useMemo(() => dashboardSummary(tasks), [tasks]);
  const groups = useMemo(() => groupTasks(tasks), [tasks]);
  const visibleGroups = groupOrder.filter(
    (group) => group !== "unknown" || summary.counts.unknown > 0,
  );
  const styles = useMemo(
    () => ({
      text: { color: theme.colors.foreground, fontSize: 14, lineHeight: 20 },
      muted: { color: theme.colors.foregroundMuted, fontSize: 12, lineHeight: 17 },
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
      taskClass: { color: theme.colors.foregroundMuted, fontSize: 11, lineHeight: 18 },
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

  async function copySearch(task: Task, id: string) {
    try {
      await copyText(attachmentKey(id, task.id));
      setNotice(
        "Search key copied. Open composer + → Watchtower task, paste the key, then select the task.",
      );
    } catch {
      setNotice("Could not copy. Use the search key below in composer + → Watchtower task.");
    }
  }

  return (
    <View>
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
                  const blocking = questionsFor(task.id, questions);
                  return (
                    <View
                      key={task.id}
                      style={[styles.task, isSelected ? styles.selectedTask : null]}
                    >
                      <Pressable
                        accessibilityRole="button"
                        accessibilityLabel={`${task.id}, ${taskTitle(task)}, ${groupLabels[group]}${task.taskClass ? `, class ${task.taskClass}` : ""}`}
                        accessibilityHint="Shows task details and attachment search"
                        accessibilityState={{ selected: isSelected, expanded: isSelected }}
                        style={styles.taskRow}
                        onPress={() => {
                          setSelection(isSelected ? null : { sourceKey, taskId: task.id });
                          setNotice("");
                        }}
                      >
                        <Text style={styles.taskId}>{task.id}</Text>
                        {task.taskClass ? (
                          <Text numberOfLines={1} style={styles.taskClass}>
                            {task.taskClass}
                          </Text>
                        ) : null}
                        <Text
                          ellipsizeMode="tail"
                          numberOfLines={isSelected ? undefined : 1}
                          style={styles.taskName}
                        >
                          {taskTitle(task)}
                        </Text>
                        <View style={[styles.badge, { borderColor: color }]}>
                          <Text style={[styles.badgeText, { color }]}>{groupLabels[group]}</Text>
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
                          {task.brief ? (
                            <View>
                              <Text style={styles.detailLabel}>Class</Text>
                              <Text selectable style={styles.text}>
                                {task.taskClass ?? "Missing; Watchtower reads it as risky."}
                              </Text>
                            </View>
                          ) : null}
                          {blocking.length ? (
                            <View>
                              <Text style={styles.detailLabel}>Open questions</Text>
                              {blocking.map((question) => (
                                <Text key={question.id} selectable style={styles.blocker}>
                                  {question.id}: {question.question}
                                </Text>
                              ))}
                            </View>
                          ) : null}
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
                          {task.brief && !task.error && workspaceId ? (
                            <>
                              <Text selectable style={styles.key}>
                                {attachmentKey(workspaceId, task.id)}
                              </Text>
                              <Pressable
                                accessibilityRole="button"
                                accessibilityLabel={`Copy attachment search for ${task.id}`}
                                style={styles.copyButton}
                                onPress={() => void copySearch(task, workspaceId)}
                              >
                                <Text style={styles.copyButtonText}>Copy attachment search</Text>
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
    </View>
  );
}
