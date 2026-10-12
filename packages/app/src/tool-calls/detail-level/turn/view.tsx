import React, {
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { AccessibilityInfo, Animated, Pressable, Text, View } from "react-native";
import { useTranslation } from "react-i18next";
import { StyleSheet } from "react-native-unistyles";
import { isWeb } from "@/constants/platform";
import { inlineUnistylesStyle } from "@/styles/unistyles-inline-style";
import type { AssistantMessageItem } from "@/types/stream";
import { describeToolCall, turnWorkCall } from "../grouping";
import type { OverviewSummary } from "../overview/model";
import type { TurnStep, TurnStepGroup, TurnStepItem, TurnStepLabel } from "./model";

const RUNNING_COLOR = "#256ABF";
const PULSE_MS = 700;
const PULSE_LOW = 0.3;
const DIMMED_OPACITY = 0.3;
const MIN_SEGMENT_SHARE = 0.04;

/** The one place that holds the label hues (oklch). */
const LABEL_HUES: Record<TurnStepLabel, number> = { explore: 155, edit: 55, verify: 300 };

/**
 * oklch to an rgb() string. Native has no oklch parser, so the design formulas
 * run here once and every platform gets plain rgb.
 */
function oklchToRgb(lightness: number, chroma: number, hue: number): string {
  const radians = (hue * Math.PI) / 180;
  const a = chroma * Math.cos(radians);
  const b = chroma * Math.sin(radians);
  const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const linear = [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
  const channels = linear.map((value) => {
    const clamped = Math.min(1, Math.max(0, value));
    const encoded = clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * clamped ** (1 / 2.4) - 0.055;
    return Math.round(encoded * 255);
  });
  return `rgb(${channels.join(", ")})`;
}

export interface TurnLabelColors {
  /** Strip segment. */
  segment: string;
  /** Label text and number. */
  text: string;
  /** Number border. */
  border: string;
}

const LABEL_COLORS: Record<TurnStepLabel, TurnLabelColors> = {
  explore: buildLabelColors(LABEL_HUES.explore),
  edit: buildLabelColors(LABEL_HUES.edit),
  verify: buildLabelColors(LABEL_HUES.verify),
};

function buildLabelColors(hue: number): TurnLabelColors {
  return {
    segment: oklchToRgb(0.74, 0.09, hue),
    text: oklchToRgb(0.52, 0.11, hue),
    border: oklchToRgb(0.8, 0.06, hue),
  };
}

export function turnLabelColors(label: TurnStepLabel): TurnLabelColors {
  return LABEL_COLORS[label];
}

export function formatTurnClock(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

export function formatTurnDuration(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  if (seconds >= 3600) {
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${Math.floor(seconds / 3600)}h ${String(minutes).padStart(2, "0")}m`;
  }
  if (seconds >= 60) {
    return `${Math.floor(seconds / 60)}m ${String(seconds % 60).padStart(2, "0")}s`;
  }
  return `${seconds}s`;
}

function sameElements<T>(a: readonly T[], b: readonly T[]): boolean {
  return a === b || (a.length === b.length && a.every((item, index) => item === b[index]));
}

function sameCounts(a: OverviewSummary, b: OverviewSummary): boolean {
  return (
    a.editedFileCount === b.editedFileCount &&
    a.commandCount === b.commandCount &&
    a.readFileCount === b.readFileCount &&
    a.searchCount === b.searchCount &&
    a.otherToolCount === b.otherToolCount &&
    a.paseoCallCount === b.paseoCallCount
  );
}

/**
 * Every streamed token builds new step objects. A step is equal when what it draws is: the
 * stream items are the same objects, so only a step whose items changed draws again.
 */
export function areTurnStepsEqual(a: TurnStep, b: TurnStep): boolean {
  return (
    a === b ||
    (a.id === b.id &&
      a.label === b.label &&
      a.isRunning === b.isRunning &&
      a.endedAt?.getTime() === b.endedAt?.getTime() &&
      sameElements(a.note, b.note) &&
      sameElements(a.items, b.items) &&
      sameElements(a.edits, b.edits) &&
      sameCounts(a.counts, b.counts))
  );
}

interface LineStats {
  additions: number;
  deletions: number;
}

const lineStatsCache = new WeakMap<object, LineStats>();

function countLines(text: string | undefined): number {
  return text ? text.replace(/\n$/, "").split("\n").length : 0;
}

function editLineStats(item: TurnStepItem): LineStats {
  const cached = lineStatsCache.get(item);
  if (cached) {
    return cached;
  }
  const call = turnWorkCall(item);
  const detail = call ? describeToolCall(call).detail : null;
  let stats: LineStats = { additions: 0, deletions: 0 };
  if (detail?.type === "edit" && detail.unifiedDiff) {
    for (const line of detail.unifiedDiff.split("\n")) {
      if (line.startsWith("+") && !line.startsWith("+++")) stats.additions += 1;
      if (line.startsWith("-") && !line.startsWith("---")) stats.deletions += 1;
    }
  } else if (detail?.type === "edit") {
    stats = { additions: countLines(detail.newString), deletions: countLines(detail.oldString) };
  } else if (detail?.type === "write") {
    stats = { additions: countLines(detail.content), deletions: 0 };
  }
  lineStatsCache.set(item, stats);
  return stats;
}

function sumLineStats(steps: readonly TurnStep[]): LineStats {
  const total: LineStats = { additions: 0, deletions: 0 };
  for (const step of steps) {
    for (const edit of step.edits) {
      const stats = editLineStats(edit);
      total.additions += stats.additions;
      total.deletions += stats.deletions;
    }
  }
  return total;
}

function shortPath(filePath: string): string {
  return filePath.split("/").filter(Boolean).slice(-2).join("/");
}

function useNow(enabled: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!enabled) {
      return undefined;
    }
    setNow(Date.now());
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [enabled]);
  return now;
}

function useReduceMotion(): boolean {
  const [reduce, setReduce] = useState(false);
  useEffect(() => {
    let active = true;
    AccessibilityInfo.isReduceMotionEnabled()
      .then((enabled) => {
        if (active) setReduce(enabled);
        return enabled;
      })
      .catch(() => undefined);
    const subscription = AccessibilityInfo.addEventListener("reduceMotionChanged", setReduce);
    return () => {
      active = false;
      subscription.remove();
    };
  }, []);
  return reduce;
}

/** An opacity value that pulses while `active`, unless the system asks for less motion. */
function usePulse(active: boolean): Animated.Value {
  const reduceMotion = useReduceMotion();
  const opacity = useRef(new Animated.Value(1)).current;
  useEffect(() => {
    if (!active || reduceMotion) {
      opacity.setValue(1);
      return undefined;
    }
    const animation = Animated.loop(
      Animated.sequence([
        Animated.timing(opacity, {
          toValue: PULSE_LOW,
          duration: PULSE_MS,
          useNativeDriver: !isWeb,
        }),
        Animated.timing(opacity, { toValue: 1, duration: PULSE_MS, useNativeDriver: !isWeb }),
      ]),
    );
    animation.start();
    return () => animation.stop();
  }, [active, opacity, reduceMotion]);
  return opacity;
}

const COUNT_KEYS = [
  ["editedFileCount", "toolCallGroup.editedFiles"],
  ["commandCount", "toolCallGroup.commands"],
  ["readFileCount", "toolCallGroup.readFiles"],
  ["searchCount", "toolCallGroup.searches"],
  ["otherToolCount", "toolCallGroup.otherTools"],
  ["paseoCallCount", "toolCallGroup.paseoCalls"],
] as const;

function useCountsText(counts: OverviewSummary): string {
  const { t } = useTranslation();
  return useMemo(
    () =>
      COUNT_KEYS.flatMap(([field, key]) => {
        const count = counts[field];
        return count > 0 ? [t(`${key}.${count === 1 ? "one" : "other"}`, { count })] : [];
      }).join(", "),
    [counts, t],
  );
}

function TurnSegment({
  step,
  index,
  weight,
  isDimmed,
  onPress,
}: {
  step: TurnStep;
  index: number;
  weight: number;
  isDimmed: boolean;
  onPress: (stepId: string) => void;
}) {
  const { t } = useTranslation();
  const pulse = usePulse(step.isRunning);
  const handlePress = useCallback(() => onPress(step.id), [onPress, step.id]);
  const color = step.isRunning ? RUNNING_COLOR : LABEL_COLORS[step.label].segment;
  const dataSet = useMemo(
    () => ({ dimmed: String(isDimmed), running: String(step.isRunning) }),
    [isDimmed, step.isRunning],
  );
  return (
    <Pressable
      testID={`turn-segment-${index}`}
      accessibilityRole="button"
      accessibilityLabel={t("toolCallGroup.turn.step", { count: index + 1 })}
      dataSet={dataSet}
      onPress={handlePress}
      style={[
        styles.segment,
        inlineUnistylesStyle({ flexGrow: weight, opacity: isDimmed ? DIMMED_OPACITY : 1 }),
      ]}
    >
      <Animated.View
        style={[
          styles.segmentFill,
          inlineUnistylesStyle({ backgroundColor: color, opacity: step.isRunning ? pulse : 1 }),
        ]}
      />
    </Pressable>
  );
}

function TurnHeader({
  group,
  selectedStepId,
  onSelect,
}: {
  group: TurnStepGroup;
  selectedStepId: string | null;
  onSelect: (stepId: string) => void;
}) {
  const { t } = useTranslation();
  const now = useNow(group.isLoading);
  const startedAt = group.run.startedAt.getTime();
  const lastEnd = group.steps.at(-1)?.endedAt?.getTime() ?? startedAt;
  const durations = group.steps.map(
    (step) => step.durationMs ?? Math.max(0, now - step.startedAt.getTime()),
  );
  const total = durations.reduce((sum, value) => sum + value, 0);
  const files = group.summary.changedFiles;
  const stats = useMemo(() => sumLineStats(group.steps), [group.steps]);
  const stepsText = group.isLoading
    ? t("toolCallGroup.turn.step", { count: group.summary.stepCount })
    : t(`toolCallGroup.turn.steps.${group.summary.stepCount === 1 ? "one" : "other"}`, {
        count: group.summary.stepCount,
      });

  return (
    <View style={styles.header}>
      <Text style={styles.title} testID="turn-title">
        {group.isLoading
          ? t("toolCallGroup.turn.working")
          : t("toolCallGroup.turn.worked", { duration: formatTurnDuration(lastEnd - startedAt) })}
      </Text>
      {group.isLoading ? (
        <Text style={styles.clock} testID="turn-clock">
          {formatTurnClock(now - startedAt)}
        </Text>
      ) : null}
      <View style={styles.strip}>
        {group.steps.map((step, index) => (
          <TurnSegment
            key={step.id}
            step={step}
            index={index}
            weight={Math.max(durations[index] ?? 0, total * MIN_SEGMENT_SHARE, 1)}
            isDimmed={selectedStepId !== null && selectedStepId !== step.id}
            onPress={onSelect}
          />
        ))}
      </View>
      <Text style={styles.summary} testID="turn-summary" numberOfLines={1}>
        {stepsText}
        {files.length > 0 ? " · " : ""}
        {files.length === 1 ? (
          <Text style={styles.fileName}>{shortPath(files[0] ?? "")}</Text>
        ) : null}
        {files.length > 1 ? t("toolCallGroup.turn.files.other", { count: files.length }) : null}
        {files.length > 0 && stats.additions + stats.deletions > 0 ? (
          <>
            {" "}
            <Text style={styles.additions}>{`+${stats.additions}`}</Text>{" "}
            <Text style={styles.deletions}>{`−${stats.deletions}`}</Text>
          </>
        ) : null}
      </Text>
    </View>
  );
}

const OUTPUT_LINES = 2;

function TurnLiveBox({ step }: { step: TurnStep }) {
  const last = step.items.at(-1);
  const call = last ? turnWorkCall(last) : undefined;
  if (!call) {
    return null;
  }
  const descriptor = describeToolCall(call);
  const isShell = descriptor.detail.type === "shell";
  const command = descriptor.detail.type === "shell" ? descriptor.detail.command : descriptor.name;
  const output = descriptor.detail.type === "shell" ? (descriptor.detail.output ?? "") : "";
  const lines = output.split("\n").filter((line) => line.trim().length > 0);
  const firstShown = Math.max(0, lines.length - OUTPUT_LINES);
  // A line number is the key: output only grows at the end, so a line keeps its number.
  const shown = lines
    .slice(firstShown)
    .map((text, offset) => ({ text, number: firstShown + offset }));
  return (
    <View style={styles.liveBox} testID="turn-live-box">
      <Text style={styles.liveCommand} numberOfLines={1}>
        {isShell ? `$ ${command}` : command}
      </Text>
      {shown.map((line) => (
        <Text key={line.number} style={styles.liveLine} numberOfLines={1}>
          {line.text}
        </Text>
      ))}
    </View>
  );
}

interface StepRowProps {
  step: TurnStep;
  index: number;
  runStartedAt: number;
  isSelected: boolean;
  onPress: (stepId: string) => void;
  registerRow: (stepId: string, node: unknown) => void;
  renderItem: (item: TurnStepItem) => ReactNode;
  renderNote: (item: AssistantMessageItem, isStreaming: boolean) => ReactNode;
}

function TurnStepRowView({
  step,
  index,
  runStartedAt,
  isSelected,
  onPress,
  registerRow,
  renderItem,
  renderNote,
}: StepRowProps) {
  const { t } = useTranslation();
  const pulse = usePulse(step.isRunning);
  const countsText = useCountsText(step.counts);
  const colors = LABEL_COLORS[step.label];
  // react-native-web already drops the click that ends a text-selecting drag.
  const handlePress = useCallback(() => onPress(step.id), [onPress, step.id]);
  const setRef = useCallback((node: unknown) => registerRow(step.id, node), [registerRow, step.id]);
  const time = step.endedAt
    ? formatTurnClock(step.endedAt.getTime() - runStartedAt)
    : t("toolCallGroup.turn.now");

  return (
    <Pressable
      ref={setRef}
      testID={`turn-step-row-${index}`}
      accessibilityRole="button"
      aria-pressed={isSelected}
      onPress={handlePress}
      style={[styles.row, isSelected && styles.rowSelected]}
    >
      <Animated.View
        style={[
          styles.number,
          inlineUnistylesStyle(
            step.isRunning
              ? { borderColor: RUNNING_COLOR, borderWidth: 1.5, opacity: pulse }
              : { borderColor: colors.border },
          ),
        ]}
      >
        <Text
          style={[
            styles.numberText,
            inlineUnistylesStyle({ color: step.isRunning ? RUNNING_COLOR : colors.text }),
          ]}
        >
          {index + 1}
        </Text>
      </Animated.View>
      <View style={styles.body}>
        <View style={styles.meta}>
          <Text
            style={[styles.label, inlineUnistylesStyle({ color: colors.text })]}
            testID={`turn-step-label-${index}`}
          >
            {t(`toolCallGroup.turn.labels.${step.label}`)}
          </Text>
          {countsText ? (
            <Text style={styles.counts} testID={`turn-step-meta-${index}`}>
              {countsText}
            </Text>
          ) : null}
        </View>
        {step.edits.map((item) => (
          <View key={item.id}>{renderItem(item)}</View>
        ))}
        {step.note.map((block, blockIndex) => (
          <View key={block.id}>
            {renderNote(block, step.isRunning && blockIndex === step.note.length - 1)}
          </View>
        ))}
        {step.isRunning ? <TurnLiveBox step={step} /> : null}
      </View>
      <Text style={styles.time} testID={`turn-step-time-${index}`}>
        {time}
      </Text>
    </Pressable>
  );
}

const TurnStepRow = memo(TurnStepRowView, (prev, next) => {
  return (
    areTurnStepsEqual(prev.step, next.step) &&
    prev.index === next.index &&
    prev.runStartedAt === next.runStartedAt &&
    prev.isSelected === next.isSelected &&
    prev.onPress === next.onPress &&
    prev.registerRow === next.registerRow &&
    prev.renderItem === next.renderItem &&
    prev.renderNote === next.renderNote
  );
});

export interface TurnStepListViewProps {
  group: TurnStepGroup;
  /** Draws an edit row: the plugin timeline renderer, or a Paseo tool call when edit diffs are off. */
  renderItem: (item: TurnStepItem) => ReactNode;
  /** Draws one Markdown block of a step note. */
  renderNote: (item: AssistantMessageItem, isStreaming: boolean) => ReactNode;
}

export const TurnStepListView = memo(function TurnStepListView({
  group,
  renderItem,
  renderNote,
}: TurnStepListViewProps) {
  // Highlight lives in the view: it belongs to this turn only and nothing else reads it.
  const [selectedStepId, setSelectedStepId] = useState<string | null>(null);
  const rows = useRef(new Map<string, unknown>());
  const toggle = useCallback((stepId: string) => {
    setSelectedStepId((current) => (current === stepId ? null : stepId));
  }, []);
  const registerRow = useCallback((stepId: string, node: unknown) => {
    if (node) {
      rows.current.set(stepId, node);
    } else {
      rows.current.delete(stepId);
    }
  }, []);

  useEffect(() => {
    const node = selectedStepId ? rows.current.get(selectedStepId) : null;
    // "nearest" scrolls only when the row is off screen.
    if (isWeb && node instanceof HTMLElement) {
      node.scrollIntoView?.({ block: "nearest", behavior: "smooth" });
    }
  }, [selectedStepId]);

  const runStartedAt = group.run.startedAt.getTime();
  return (
    <View style={styles.container} testID="turn-step-list">
      <TurnHeader group={group} selectedStepId={selectedStepId} onSelect={toggle} />
      <View style={styles.list}>
        <View style={styles.rail} />
        {group.steps.map((step, index) => (
          <TurnStepRow
            key={step.id}
            step={step}
            index={index}
            runStartedAt={runStartedAt}
            isSelected={selectedStepId === step.id}
            onPress={toggle}
            registerRow={registerRow}
            renderItem={renderItem}
            renderNote={renderNote}
          />
        ))}
      </View>
      {group.isLoading ? null : <View style={styles.rule} />}
    </View>
  );
});

const MONO_LABEL_SIZE = 10;

const styles = StyleSheet.create((theme) => ({
  container: {
    paddingVertical: theme.spacing[2],
  },
  header: {
    flexDirection: "row",
    alignItems: "center",
    gap: 12,
  },
  title: {
    fontSize: 13.5,
    fontWeight: "600",
    color: theme.colors.foreground,
  },
  clock: {
    fontFamily: theme.fontFamily.mono,
    fontSize: 12.5,
    color: theme.colors.foregroundMuted,
  },
  strip: {
    flexDirection: "row",
    gap: 2,
    width: 170,
    height: 8,
    borderRadius: 3,
    overflow: "hidden",
    flexShrink: 0,
  },
  segment: {
    height: 8,
    flexBasis: 0,
  },
  segmentFill: {
    flex: 1,
  },
  summary: {
    marginLeft: "auto",
    flexShrink: 1,
    fontSize: 13,
    color: theme.colors.foregroundMuted,
  },
  fileName: {
    fontFamily: theme.fontFamily.mono,
    fontSize: 12,
    color: theme.colors.foreground,
  },
  additions: {
    color: theme.colors.statusSuccess,
  },
  deletions: {
    color: theme.colors.statusDanger,
  },
  list: {
    position: "relative",
    marginTop: 16,
    gap: 6,
  },
  rail: {
    position: "absolute",
    left: 10,
    top: 12,
    bottom: 12,
    width: 1,
    backgroundColor: theme.colors.border,
  },
  // Padding plus an equal negative margin gives the highlight its 6px spread without moving content.
  row: {
    flexDirection: "row",
    alignItems: "flex-start",
    gap: 12,
    marginHorizontal: -6,
    paddingHorizontal: 6,
    paddingVertical: 4,
    borderRadius: 4,
  },
  rowSelected: {
    backgroundColor: theme.colors.surface2,
  },
  number: {
    width: 20,
    height: 20,
    marginRight: 2,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: theme.colors.border,
    backgroundColor: theme.colors.surface0,
    alignItems: "center",
    justifyContent: "center",
  },
  numberText: {
    fontFamily: theme.fontFamily.mono,
    fontSize: MONO_LABEL_SIZE,
    fontWeight: "600",
  },
  body: {
    flex: 1,
    minWidth: 0,
    gap: 3,
  },
  meta: {
    flexDirection: "row",
    alignItems: "baseline",
    gap: 6,
  },
  label: {
    fontFamily: theme.fontFamily.mono,
    fontSize: MONO_LABEL_SIZE,
    fontWeight: "600",
    letterSpacing: 0.6,
    textTransform: "uppercase",
  },
  counts: {
    fontSize: 12.5,
    color: theme.colors.foregroundMuted,
  },
  time: {
    width: 40,
    fontFamily: theme.fontFamily.mono,
    fontSize: 11.5,
    lineHeight: 20,
    color: theme.colors.foregroundMuted,
    textAlign: "right",
  },
  liveBox: {
    marginTop: 6,
    borderWidth: 1,
    borderColor: theme.colors.border,
    borderRadius: 8,
    paddingHorizontal: 10,
    paddingVertical: 7,
  },
  liveCommand: {
    fontFamily: theme.fontFamily.mono,
    fontSize: 12,
    lineHeight: 19,
    color: theme.colors.foreground,
  },
  liveLine: {
    fontFamily: theme.fontFamily.mono,
    fontSize: 12,
    lineHeight: 19,
    color: theme.colors.foregroundMuted,
  },
  rule: {
    height: 1,
    marginTop: 22,
    marginBottom: 18,
    backgroundColor: theme.colors.border,
  },
}));
