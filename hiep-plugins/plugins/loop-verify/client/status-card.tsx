import type { PluginTimelineItemProps } from "@getpaseo/plugin/client";
import { useMemo } from "react";
import { Text, View } from "react-native";
import type { LoopStatus } from "../shared/status";

type Theme = PluginTimelineItemProps["theme"];

function statusLine(data: LoopStatus): string {
  switch (data.status) {
    case "passed":
      return `Verify passed at round ${data.round}`;
    case "paused":
      return "Paused: the agent asked a question. Reply to it to resume.";
    case "exhausted":
      return `Verify still failing after ${data.maxRounds} rounds`;
    case "stopped":
      return "Loop stopped";
    case "running":
      return `Verify failed. Round ${data.round + 1} started in a new agent.`;
  }
}

function statusColor(status: LoopStatus["status"], theme: Theme): string {
  if (status === "passed") return theme.colors.statusSuccess;
  if (status === "paused") return theme.colors.statusWarning;
  if (status === "exhausted") return theme.colors.statusDanger;
  return theme.colors.foregroundMuted;
}

export function LoopStatusCard({ item, theme }: PluginTimelineItemProps<LoopStatus>) {
  const data = item.data;
  const styles = useMemo(
    () => ({
      card: {
        gap: 6,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 10,
        padding: 12,
        backgroundColor: theme.colors.surface1,
      },
      header: { flexDirection: "row" as const, justifyContent: "space-between" as const },
      title: { color: theme.colors.foreground, fontWeight: "600" as const },
      muted: { color: theme.colors.foregroundMuted },
      status: { color: statusColor(data.status, theme), fontWeight: "600" as const },
      output: {
        color: theme.colors.foregroundMuted,
        fontFamily: "monospace",
        fontSize: 12,
        backgroundColor: theme.colors.surface0,
        borderRadius: 6,
        padding: 8,
      },
    }),
    [theme, data.status],
  );
  const exit = data.exitCode === null ? "no exit code" : `exit ${data.exitCode}`;
  const output = data.output.trim();

  return (
    <View style={styles.card}>
      <View style={styles.header}>
        <Text style={styles.title}>Loop verify</Text>
        <Text style={styles.muted}>
          Round {data.round}/{data.maxRounds}
        </Text>
      </View>
      <Text style={styles.status}>{statusLine(data)}</Text>
      <Text style={styles.muted}>
        {data.verify} · {exit}
      </Text>
      {data.status !== "passed" && output ? (
        <Text style={styles.output} numberOfLines={8}>
          {output}
        </Text>
      ) : null}
    </View>
  );
}
