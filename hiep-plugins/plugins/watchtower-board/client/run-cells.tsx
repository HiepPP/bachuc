import { Text, View } from "react-native";
import type { OverviewRun, OverviewTask } from "../shared/overview";
import { taskTitle } from "./dashboard";
import { Empty, MONO, OverviewCell, type ThemeProps } from "./overview-cells";
import type { Timeline } from "./timeline";
import { clockAt, shortId } from "./timeline-geometry";

// Presentational parts for the autorun state. They take data through props and never fetch.

export function AutorunStrip({ run, theme }: { run: OverviewRun | null } & ThemeProps) {
  if (!run) return null;
  const { colors } = theme;
  const state = run.finished ? "Run finished" : run.stopped ? "Run stopped" : "Autorun running";
  const live = !run.finished && !run.stopped;
  const color = live
    ? colors.statusSuccess
    : run.stopped
      ? colors.statusWarning
      : colors.foreground;
  const last = [...run.log].reverse().find((row) => row.endMinute !== null);
  const items = [
    run.runner ? ["runner", run.runner] : null,
    run.started ? ["started", run.started] : null,
    ["iterations", String(run.log.length)],
    last ? ["last activity", clockAt(run.started, last.endMinute ?? 0) ?? last.end] : null,
    run.finished ? ["finished", run.finished] : null,
  ].filter((item): item is string[] => item !== null);
  return (
    <View
      accessibilityRole="summary"
      style={{
        flexDirection: "row",
        flexWrap: "wrap",
        alignItems: "center",
        columnGap: 14,
        rowGap: 4,
        paddingHorizontal: 12,
        paddingVertical: 8,
        borderRadius: 8,
        borderWidth: 1,
        borderColor: colors.border,
        backgroundColor: colors.surface1,
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: 6 }}>
        <View style={{ width: 8, height: 8, borderRadius: 4, backgroundColor: color }} />
        <Text style={{ fontSize: 12, fontWeight: "700", color }}>{state}</Text>
      </View>
      {items.map(([label, value]) => (
        <Text key={label} style={{ fontSize: 12, color: colors.foreground }}>
          <Text style={{ color: colors.foregroundMuted }}>{label} </Text>
          {value}
        </Text>
      ))}
    </View>
  );
}

export function RunningNowCell({
  timeline,
  tasks,
  started,
  theme,
}: {
  timeline: Timeline;
  tasks: readonly OverviewTask[];
  started: string | null;
} & ThemeProps) {
  const { colors } = theme;
  const running = timeline.bars.filter((bar) => bar.kind === "running");
  const average = timeline.stats.averageMinutes;
  return (
    <View
      style={{
        borderWidth: 1,
        borderColor: running.length ? colors.foreground : colors.border,
        borderRadius: 12,
        padding: 14,
        gap: 10,
        backgroundColor: colors.surface0,
      }}
    >
      <Text
        style={{
          fontSize: 11,
          fontWeight: "600",
          letterSpacing: 0.6,
          textTransform: "uppercase",
          color: colors.foregroundMuted,
        }}
      >
        Running now
      </Text>
      {running.length === 0 ? <Empty text="Nothing running." theme={theme} /> : null}
      {running.map((bar) => {
        const task = tasks.find((entry) => entry.id === bar.id);
        const minutes = Math.round(bar.end - bar.start);
        const over = minutes > average;
        const since = clockAt(started, bar.start);
        return (
          <View key={bar.id} style={{ gap: 6 }}>
            <View style={{ flexDirection: "row", alignItems: "baseline", gap: 8 }}>
              <Text style={{ fontFamily: MONO, fontSize: 12, color: colors.foreground }}>
                {bar.id}
              </Text>
              <Text
                numberOfLines={1}
                style={{ flex: 1, fontSize: 15, fontWeight: "600", color: colors.foreground }}
              >
                {task ? taskTitle(task) : ""}
              </Text>
            </View>
            <View
              accessible
              accessibilityLabel={`${minutes} of about ${average} minutes`}
              style={{ height: 6, borderRadius: 3, backgroundColor: colors.surface2 }}
            >
              <View
                style={{
                  width: `${Math.min(100, (minutes / Math.max(average, 1)) * 100)}%`,
                  height: 6,
                  borderRadius: 3,
                  backgroundColor: over ? colors.statusWarning : colors.foreground,
                }}
              />
            </View>
            <Text
              style={{ fontSize: 12, color: over ? colors.statusWarning : colors.foregroundMuted }}
            >
              {minutes} min of ~{average}
              {over ? `, ${minutes - average} min over` : ""}
              {since ? ` · started ${since}` : ""}
            </Text>
          </View>
        );
      })}
    </View>
  );
}

export function UpNextCell({
  timeline,
  tasks,
  started,
  theme,
}: {
  timeline: Timeline;
  tasks: readonly OverviewTask[];
  started: string | null;
} & ThemeProps) {
  const { colors } = theme;
  const next = timeline.bars
    .filter((bar) => bar.kind === "planned" || bar.kind === "blocked")
    .sort((a, b) => a.start - b.start);
  return (
    <OverviewCell title="Up next" aside={next.length ? "in order" : undefined} theme={theme}>
      {next.length === 0 ? <Empty text="Nothing planned." theme={theme} /> : null}
      {next.map((bar) => {
        const task = tasks.find((entry) => entry.id === bar.id);
        const blocked = bar.kind === "blocked";
        const at = clockAt(started, bar.start);
        return (
          <View
            key={bar.id}
            style={{ flexDirection: "row", alignItems: "center", gap: 8, paddingVertical: 4 }}
          >
            <View
              style={{
                width: 8,
                height: 8,
                borderRadius: 4,
                borderWidth: 1,
                borderColor: blocked ? colors.statusDanger : colors.foregroundMuted,
                backgroundColor: blocked ? colors.statusDanger : "transparent",
              }}
            />
            <Text style={{ fontFamily: MONO, fontSize: 11, color: colors.foregroundMuted }}>
              {shortId(bar.id)}
            </Text>
            <Text numberOfLines={1} style={{ flex: 1, fontSize: 13, color: colors.foreground }}>
              {task ? taskTitle(task) : bar.id}
            </Text>
            <Text
              style={{
                fontSize: 12,
                color: blocked ? colors.statusDanger : colors.foregroundMuted,
              }}
            >
              {blocked ? "blocked" : at ? `~${at}` : ""}
            </Text>
          </View>
        );
      })}
    </OverviewCell>
  );
}
