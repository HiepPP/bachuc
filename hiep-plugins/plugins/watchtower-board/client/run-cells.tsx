import { Text, View } from "react-native";
import type { OverviewRun } from "../shared/overview";
import type { ThemeProps } from "./overview-cells";
import { startedAt } from "./timeline";

const DAY_MINUTES = 24 * 60;

// The clock time of a timeline minute, from the run's `Started:` value.
export function clockAt(started: string | null, minute: number): string | null {
  const origin = startedAt(started);
  if (!origin) return null;
  const total = (((origin.minutes + Math.round(minute)) % DAY_MINUTES) + DAY_MINUTES) % DAY_MINUTES;
  return `${String(Math.floor(total / 60)).padStart(2, "0")}:${String(total % 60).padStart(2, "0")}`;
}

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
