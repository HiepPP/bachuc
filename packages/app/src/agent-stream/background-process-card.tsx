import React, { createContext, memo, useCallback, useContext, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { useTranslation } from "react-i18next";
import { StyleSheet } from "react-native-unistyles";
import type { AgentBackgroundProcess } from "@getpaseo/protocol/messages";
import { LiveElapsed } from "@/components/message";
import { Button } from "@/components/ui/button";
import { formatTurnDuration } from "@/tool-calls/detail-level/turn/view";
import {
  EMPTY_BACKGROUND_PROCESS_PLACEMENT,
  type BackgroundProcessPlacement,
} from "./background-process-placement";

export interface BackgroundProcessContextValue {
  placement: BackgroundProcessPlacement;
  /** Null when the host cannot stop a process. */
  onStop: ((processId: string) => void) | null;
}

export const BackgroundProcessContext = createContext<BackgroundProcessContextValue>({
  placement: EMPTY_BACKGROUND_PROCESS_PLACEMENT,
  onStop: null,
});

/**
 * Reads its placement from context, so a memoized history row picks up process changes without
 * the stream redrawing the row.
 */
export function BackgroundProcessHostSlot({ hostId }: { hostId: string }) {
  const { placement, onStop } = useContext(BackgroundProcessContext);
  const slot = placement.byHostId.get(hostId);
  if (!slot) return null;
  return (
    <BackgroundProcessCard
      processes={slot.processes}
      movedCount={slot.movedCount}
      onStop={onStop}
    />
  );
}

export const BackgroundProcessCard = memo(function BackgroundProcessCard({
  processes,
  movedCount,
  onStop,
}: {
  processes: readonly AgentBackgroundProcess[];
  movedCount: number;
  onStop: ((processId: string) => void) | null;
}) {
  const { t } = useTranslation();
  const running = processes.filter((process) => process.status === "running");
  const finished = processes.filter((process) => process.status !== "running");
  return (
    <View style={styles.container} testID="background-process-card">
      {running.length > 0 ? (
        <View style={styles.card}>
          <View style={styles.header}>
            <View style={[styles.dot, styles.dotRunning]} />
            <Text style={styles.title}>{t("backgroundProcesses.title")}</Text>
            <Text style={styles.muted}>
              {t("backgroundProcesses.running", { count: running.length })}
            </Text>
          </View>
          {running.map((process) => (
            <RunningRow key={process.id} process={process} onStop={onStop} />
          ))}
        </View>
      ) : null}
      {finished.length > 0 ? (
        <View style={styles.card}>
          {finished.map((process) => (
            <FinishedRow key={process.id} process={process} />
          ))}
        </View>
      ) : null}
      {movedCount > 0 ? (
        <View style={styles.moved}>
          <View style={[styles.dot, styles.dotRunning]} />
          <Text style={styles.movedText}>
            {t("backgroundProcesses.moved", { count: movedCount })}
          </Text>
        </View>
      ) : null}
    </View>
  );
});

function processLabel(process: AgentBackgroundProcess): string {
  return process.command ?? process.description;
}

function KindLabel({ kind }: { kind: string }) {
  const { t } = useTranslation();
  return (
    <Text style={styles.kind}>
      {kind === "monitor"
        ? t("backgroundProcesses.kindMonitor")
        : t("backgroundProcesses.kindBash")}
    </Text>
  );
}

const RunningRow = memo(function RunningRow({
  process,
  onStop,
}: {
  process: AgentBackgroundProcess;
  onStop: ((processId: string) => void) | null;
}) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const toggle = useCallback(() => setExpanded((value) => !value), []);
  const stop = useCallback(() => onStop?.(process.id), [onStop, process.id]);
  const lastLine = process.tail[process.tail.length - 1];
  return (
    <View style={styles.row}>
      <Pressable onPress={toggle} style={styles.rowMain} accessibilityRole="button">
        <View style={[styles.dot, styles.dotRunning]} />
        <KindLabel kind={process.kind} />
        <Text style={styles.command} numberOfLines={1}>
          {processLabel(process)}
        </Text>
        <LiveElapsed startedAt={new Date(process.startedAt)} style={styles.elapsed} />
      </Pressable>
      {onStop ? (
        <Button size="xs" variant="outline" onPress={stop}>
          {t("backgroundProcesses.stop")}
        </Button>
      ) : null}
      {expanded ? (
        <ProcessLog process={process} />
      ) : (
        <Text style={styles.tail} numberOfLines={1}>
          {lastLine ?? t("backgroundProcesses.noOutput")}
        </Text>
      )}
    </View>
  );
});

/** Tail lines can repeat, so a line's key is its text plus how often that text came before. */
function keyedLines(lines: readonly string[]): { key: string; line: string }[] {
  const seen = new Map<string, number>();
  return lines.map((line) => {
    const count = seen.get(line) ?? 0;
    seen.set(line, count + 1);
    return { key: `${count}:${line}`, line };
  });
}

function ProcessLog({ process }: { process: AgentBackgroundProcess }) {
  const { t } = useTranslation();
  return (
    <View style={styles.log}>
      {process.tail.length > 0 ? (
        keyedLines(process.tail).map(({ key, line }) => (
          <Text key={key} style={styles.logLine} numberOfLines={1}>
            {line}
          </Text>
        ))
      ) : (
        <Text style={styles.logLine}>{t("backgroundProcesses.noOutput")}</Text>
      )}
      {process.outputFile ? (
        <Text style={styles.path} numberOfLines={1} selectable>
          {process.outputFile}
        </Text>
      ) : null}
    </View>
  );
}

type ResultTone = "success" | "danger" | "muted";

function finishedResult(
  process: AgentBackgroundProcess,
  t: (key: string, options?: Record<string, unknown>) => string,
): { text: string; tone: ResultTone } {
  if (process.status === "stopped")
    return { text: t("backgroundProcesses.stopped"), tone: "muted" };
  const failed = process.status === "failed" || (process.exitCode ?? 0) !== 0;
  if (process.exitCode !== null) {
    return {
      text: t("backgroundProcesses.exit", { code: process.exitCode }),
      tone: failed ? "danger" : "success",
    };
  }
  return failed
    ? { text: t("backgroundProcesses.failed"), tone: "danger" }
    : { text: t("backgroundProcesses.done"), tone: "success" };
}

const FinishedRow = memo(function FinishedRow({ process }: { process: AgentBackgroundProcess }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const toggle = useCallback(() => setExpanded((value) => !value), []);
  const result = finishedResult(process, t);
  const durationMs =
    process.endedAt === null ? null : Date.parse(process.endedAt) - Date.parse(process.startedAt);
  const dotStyle = { success: styles.dotSuccess, danger: styles.dotDanger, muted: styles.dotMuted }[
    result.tone
  ];
  const resultStyle = {
    success: styles.resultSuccess,
    danger: styles.resultDanger,
    muted: styles.muted,
  }[result.tone];
  return (
    <View style={styles.row}>
      <Pressable onPress={toggle} style={styles.rowMain} accessibilityRole="button">
        <View style={[styles.dot, dotStyle]} />
        <KindLabel kind={process.kind} />
        <Text style={styles.command} numberOfLines={1}>
          {processLabel(process)}
        </Text>
        <Text style={[styles.elapsed, resultStyle]}>{result.text}</Text>
        {durationMs === null ? null : (
          <Text style={[styles.elapsed, styles.muted]}>{formatTurnDuration(durationMs)}</Text>
        )}
      </Pressable>
      {expanded ? <ProcessLog process={process} /> : null}
    </View>
  );
});

const DOT_SIZE = 7;

const styles = StyleSheet.create((theme) => ({
  container: {
    gap: 8,
    marginTop: 14,
  },
  card: {
    borderWidth: 1,
    borderColor: theme.colors.border,
    borderRadius: 8,
    backgroundColor: theme.colors.surface0,
    overflow: "hidden",
  },
  header: {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    paddingHorizontal: 12,
    paddingVertical: 9,
    borderBottomWidth: 1,
    borderBottomColor: theme.colors.border,
  },
  title: {
    fontSize: 12.5,
    fontWeight: "600",
    color: theme.colors.foreground,
  },
  muted: {
    fontSize: 12.5,
    color: theme.colors.foregroundMuted,
  },
  row: {
    flexDirection: "row",
    flexWrap: "wrap",
    alignItems: "center",
    columnGap: 9,
    paddingHorizontal: 12,
    paddingVertical: 8,
  },
  rowMain: {
    flex: 1,
    minWidth: 0,
    flexDirection: "row",
    alignItems: "center",
    gap: 9,
  },
  dot: {
    width: DOT_SIZE,
    height: DOT_SIZE,
    borderRadius: DOT_SIZE / 2,
  },
  dotRunning: { backgroundColor: theme.colors.statusDotRunning },
  dotSuccess: { backgroundColor: theme.colors.statusDotSuccess },
  dotDanger: { backgroundColor: theme.colors.statusDotDanger },
  dotMuted: { backgroundColor: theme.colors.foregroundMuted },
  kind: {
    width: 58,
    fontFamily: theme.fontFamily.mono,
    fontSize: 9.5,
    fontWeight: "600",
    letterSpacing: 0.6,
    textTransform: "uppercase",
    color: theme.colors.foregroundMuted,
  },
  command: {
    flex: 1,
    minWidth: 0,
    fontFamily: theme.fontFamily.mono,
    fontSize: 12.5,
    color: theme.colors.foreground,
  },
  elapsed: {
    fontFamily: theme.fontFamily.mono,
    fontSize: 12,
    color: theme.colors.foreground,
  },
  resultSuccess: { color: theme.colors.statusSuccess },
  resultDanger: { color: theme.colors.statusDanger },
  // Tail and log sit under the command column: dot, gap, kind, gap.
  tail: {
    flexBasis: "100%",
    marginTop: 3,
    paddingLeft: DOT_SIZE + 9 + 58 + 9,
    fontFamily: theme.fontFamily.mono,
    fontSize: 11.5,
    color: theme.colors.foregroundMuted,
  },
  log: {
    flexBasis: "100%",
    marginTop: 6,
    marginLeft: DOT_SIZE + 9 + 58 + 9,
    borderWidth: 1,
    borderColor: theme.colors.border,
    borderRadius: 6,
    backgroundColor: theme.colors.surface1,
    paddingHorizontal: 9,
    paddingVertical: 7,
    gap: 1,
  },
  logLine: {
    fontFamily: theme.fontFamily.mono,
    fontSize: 11,
    lineHeight: 17,
    color: theme.colors.foreground,
  },
  path: {
    marginTop: 4,
    fontFamily: theme.fontFamily.mono,
    fontSize: 11,
    color: theme.colors.foregroundMuted,
  },
  moved: {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
  },
  movedText: {
    fontSize: 12,
    color: theme.colors.foregroundMuted,
  },
}));
