import { useMemo } from "react";
import { Text, View } from "react-native";
import type { Board, Question, Run, Task } from "../shared/board";
import type { OverviewRunEntry } from "../shared/overview";
import { dashboardSummary, plural, type TaskGroup, taskGroup } from "./dashboard";
import { Empty, MONO, OverviewCell, ShowMore, type ThemeProps, useLimit } from "./overview-cells";
import { TaskList } from "./task-list";

// The parts of the former Explorer board. The Watchtower page shows them in its left rail, and
// the new workspace side panel shows them alone. They take data through props and never fetch.

export function ProgressCard({ tasks, theme }: { tasks: readonly Task[] } & ThemeProps) {
  const { colors } = theme;
  const summary = useMemo(() => dashboardSummary(tasks), [tasks]);
  // Running work takes the accent, work that waits on the owner the warning color (mock D v2).
  const segmentColor: Record<TaskGroup, string> = {
    done: colors.statusSuccess,
    active: colors.accent,
    blocked: colors.statusWarning,
    todo: colors.surface2,
    unknown: colors.surface2,
  };
  const { counts } = summary;
  const countLine = [
    `${counts.done} done`,
    counts.active > 0 ? `${counts.active} running` : null,
    counts.blocked > 0 ? `${counts.blocked} waiting` : null,
    `${counts.todo + counts.unknown} not started`,
  ]
    .filter(Boolean)
    .join(" · ");
  return (
    <View
      accessible
      accessibilityLabel={`${summary.percentage}% complete, ${summary.counts.done} of ${summary.total} done`}
      style={{
        gap: 10,
        padding: 14,
        borderWidth: 1,
        borderColor: colors.border,
        borderRadius: 12,
        backgroundColor: colors.surface0,
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "baseline", gap: 12 }}>
        <Text style={{ fontSize: 26, fontWeight: "700", color: colors.foreground }}>
          {summary.percentage}%
        </Text>
        <Text style={{ marginLeft: "auto", fontSize: 12, color: colors.foregroundMuted }}>
          {summary.counts.done} of {summary.total} done
        </Text>
      </View>
      <View style={{ height: 6, flexDirection: "row", gap: 2 }}>
        {tasks.map((task) => (
          <View
            key={task.id}
            style={{
              flex: 1,
              minWidth: 1,
              borderRadius: 2,
              backgroundColor: segmentColor[taskGroup(task.status)],
            }}
          />
        ))}
      </View>
      <Text style={{ fontFamily: MONO, fontSize: 11, color: colors.foregroundMuted }}>
        {countLine}
      </Text>
    </View>
  );
}

export function WaitingCard({
  questions,
  proposedAdrs,
  theme,
}: { questions: readonly Question[]; proposedAdrs: number } & ThemeProps) {
  if (questions.length === 0 && proposedAdrs === 0) return null;
  const { colors } = theme;
  return (
    <OverviewCell title="Waiting on the owner" theme={theme}>
      <Text style={{ fontSize: 12, color: colors.foregroundMuted, marginBottom: 6 }}>
        {plural(questions.length, "blocking question")} · {plural(proposedAdrs, "proposed ADR")}
      </Text>
      {questions.map((question) => (
        <View key={question.id} style={{ marginTop: 4 }}>
          <Text style={{ fontSize: 12, fontWeight: "600", color: colors.foreground }}>
            {question.id} · blocks {question.blocks.join(", ")}
          </Text>
          <Text selectable style={{ fontSize: 13, lineHeight: 19, color: colors.foreground }}>
            {question.question}
          </Text>
        </View>
      ))}
    </OverviewCell>
  );
}

export function TasksCard({
  board,
  error,
  workspaceId,
  sourceKey,
  initialTaskId,
  selectedTaskId,
  onSelectTask,
  onRevealSelected,
  theme,
}: {
  board: Board | undefined;
  error: Error | null;
  workspaceId: string | null;
  sourceKey: string;
  initialTaskId?: string;
  // Pass these to control the selection; see `TaskList`.
  selectedTaskId?: string | null;
  onSelectTask?(taskId: string | null): void;
  onRevealSelected?(row: View): void;
} & ThemeProps) {
  const { colors } = theme;
  return (
    <OverviewCell
      title="Tasks"
      aside={board ? String(board.tasks.length) : undefined}
      theme={theme}
    >
      {error ? (
        <Text style={{ fontSize: 12.5, color: colors.statusDanger }}>
          Could not load tasks. {error.message}
        </Text>
      ) : !board ? (
        <Empty text="Loading tasks…" theme={theme} />
      ) : board.tasks.length === 0 ? (
        <Empty text={board.message ?? "No tasks."} theme={theme} />
      ) : (
        <TaskList
          tasks={board.tasks}
          questions={board.questions}
          workspaceId={workspaceId}
          sourceKey={sourceKey}
          initialTaskId={initialTaskId}
          selectedTaskId={selectedTaskId}
          onSelectTask={onSelectTask}
          onRevealSelected={onRevealSelected}
          dense
          theme={theme}
          collapsedGroups={["done"]}
        />
      )}
    </OverviewCell>
  );
}

type LogRow = Pick<OverviewRunEntry, "start" | "end" | "task" | "result" | "detail">;

// Rows newest first. A dated time keeps only its clock part, so it fits its column.
export function AutorunLogCard({
  run,
  rows,
  theme,
}: { run: Run | null; rows: readonly LogRow[] } & ThemeProps) {
  const { colors } = theme;
  const { shown, more } = useLimit(rows);
  if (!run) return null;
  const clock = (value: string) => value.replace(/^\d{4}-\d{2}-\d{2}[ T]/, "");
  const mono = { fontFamily: MONO, fontSize: 11, lineHeight: 17, color: colors.foregroundMuted };
  return (
    <OverviewCell title="Autorun" aside={plural(run.total, "iteration")} theme={theme}>
      <Text selectable style={{ fontSize: 12, lineHeight: 17, color: colors.foregroundMuted }}>
        {[
          run.runner && `Runner ${run.runner}`,
          run.profile && `Profile ${run.profile}`,
          run.schedule && `Schedule ${run.schedule}`,
        ]
          .filter(Boolean)
          .join(" · ") || "Runner not recorded"}
      </Text>
      <Text
        style={{ fontSize: 12, lineHeight: 17, color: colors.foregroundMuted, marginBottom: 6 }}
      >
        Started {run.started ?? "-"} · {run.finished ? `Finished ${run.finished}` : "Not finished"}
      </Text>
      {rows.length === 0 ? <Empty text="No iterations logged yet." theme={theme} /> : null}
      {shown.map((row, index) => (
        <View
          key={`${index}-${row.task}-${row.start}`}
          style={{ paddingVertical: 4, borderTopWidth: 1, borderTopColor: colors.border }}
        >
          <View style={{ flexDirection: "row", gap: 6 }}>
            <Text style={{ ...mono, width: 40 }}>{clock(row.start)}</Text>
            <Text style={{ ...mono, width: 40 }}>{clock(row.end)}</Text>
            <Text style={{ ...mono, width: 30 }}>{row.task.replace(/^TASK-/, "")}</Text>
            <Text numberOfLines={1} style={{ ...mono, flex: 1, color: colors.foreground }}>
              {row.result.toLowerCase()}
            </Text>
          </View>
          {row.detail ? (
            <Text selectable numberOfLines={2} style={mono}>
              {row.detail}
            </Text>
          ) : null}
        </View>
      ))}
      <ShowMore {...more} theme={theme} />
    </OverviewCell>
  );
}
