import type { PluginSurfaceProps, PluginHostSummary } from "@getpaseo/plugin/client";
import { useHosts, useSettings } from "@getpaseo/plugin/client";
import { Icon, ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { memo, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  ActivityIndicator,
  Animated,
  Easing,
  Platform,
  Pressable,
  Text,
  View,
  type TextStyle,
  type ViewStyle,
} from "react-native";
import {
  boardPollInterval,
  boardRpc,
  editDiffsModeRpc,
  removeRunRpc,
  starRunRpc,
  groupRuns,
  type BoardRun,
} from "../shared/board";
import { boardColumns, type RunTree } from "../shared/tree";
import { BoardSizeControl } from "./size-control";
import { BoardViewSwitch, RecapsView } from "./recaps";
import { BOARD_SIZE_DEFAULT, boardScale, boardSize, clampBoardSize } from "../shared/board-size";
import { effortColor } from "../shared/effort";
import { allocateColors, projectColors } from "../shared/project-colors";
import { boardConnectionState } from "./connection";
import { createBoardHostClient, type BoardHostClient } from "./hosts";
import { useHostSettings } from "./host-settings";
import { AgentAvatar } from "./avatar";
import { Orb, OrbAvatar, orbSupported } from "./orb";
import { orbSettings, type OrbSettings } from "../shared/orb";
import { lastSettings } from "./warm";
import { useClock } from "./clock";
import { closeRemovedRuntimes, removeFinishedRun } from "./remove";
import { reducedMotion } from "./motion";
import { focusRing, keyboardFocus } from "./focus";
import { repoDivider, repoLabel, repoMark, repoSurface } from "./repo-color";

const SECOND = 1_000;
// Shape lock: cards and notices 8; controls and subagent rows 6.
const CARD_RADIUS = 8;
const CONTROL_RADIUS = 6;
const COLUMN_GAP = 24;
const RAIL_HEIGHT = 40;
const SEPARATOR = " · ";
// Content of a finished card that was opened since; actions keep full contrast.
const SEEN_OPACITY = 0.5;
// Counts and durations keep their width as digits change.
const TABULAR = { fontVariant: ["tabular-nums"] } satisfies TextStyle;

/** Web keeps each host rail in view while its lane scrolls. */
const STICKY: ViewStyle | null =
  Platform.OS === "web" ? ({ position: "sticky", top: 0 } as unknown as ViewStyle) : null;
// The card draws the ring for its open action, so the browser's own outline stays off.
const NO_OUTLINE = { outlineWidth: 0 } satisfies ViewStyle;

function timeValue(value: string | null): number | null {
  if (!value) return null;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function durationLabel(milliseconds: number | null): string | null {
  if (milliseconds === null || milliseconds < 0) return null;
  const seconds = Math.floor(milliseconds / SECOND);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ${minutes % 60}m`;
  const days = Math.floor(hours / 24);
  return `${days}d ${hours % 24}h`;
}

function relativeLabel(value: string | null, now: number): string {
  const time = timeValue(value);
  if (time === null) return "Finish time unavailable";
  const elapsed = Math.max(0, now - time);
  if (elapsed < 10 * SECOND) return "Just now";
  return `${durationLabel(elapsed) ?? "0s"} ago`;
}

function statusLabel(status: BoardRun["status"]): string {
  if (status === "unknown") return "Outcome unknown";
  return status.charAt(0).toUpperCase() + status.slice(1);
}

function runDuration(run: BoardRun, now: number): string | null {
  if (run.status === "unknown") return null;
  const started = timeValue(run.startedAt);
  if (started === null) return null;
  const ended = run.status === "running" ? now : timeValue(run.endedAt);
  if (ended === null) return null;
  return durationLabel(ended - started);
}

/** Host spinner scaled to the status row; the single loading indicator for a running card. */
function Spinner({ color, size }: { color: string; size: number }) {
  return (
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      style={{ width: size, height: size, alignItems: "center", justifyContent: "center" }}
    >
      <ActivityIndicator size="small" color={color} style={{ transform: [{ scale: size / 20 }] }} />
    </View>
  );
}

/** Looping pulse that draws the eye to runs waiting for the user. */
function AttentionPulse({
  children,
  style,
  grow = 1.08,
}: {
  children?: ReactNode;
  style?: ViewStyle;
  grow?: number;
}) {
  const [pulse] = useState(() => new Animated.Value(0));
  useEffect(() => {
    if (reducedMotion()) return;
    const step = (toValue: number) =>
      Animated.timing(pulse, {
        toValue,
        duration: 700,
        easing: Easing.inOut(Easing.quad),
        useNativeDriver: Platform.OS !== "web",
      });
    const loop = Animated.loop(Animated.sequence([step(1), step(0)]));
    loop.start();
    return () => loop.stop();
  }, [pulse]);
  return (
    <Animated.View
      pointerEvents="none"
      style={[
        style,
        {
          opacity: pulse.interpolate({ inputRange: [0, 1], outputRange: [1, 0.35] }),
          transform: [{ scale: pulse.interpolate({ inputRange: [0, 1], outputRange: [1, grow] }) }],
        },
      ]}
    >
      {children}
    </Animated.View>
  );
}

/** Project mark matching the sidebar: rounded square with the project's initial. */
function ProjectMark({
  name,
  color,
  size,
  theme,
}: {
  name: string;
  color: string;
  size: number;
  theme: PluginSurfaceProps["theme"];
}) {
  return (
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      style={{
        width: size,
        height: size,
        borderRadius: size * 0.3,
        backgroundColor: color,
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
      }}
    >
      <Text
        style={{
          color: theme.colors.surface1,
          fontSize: size * 0.55,
          lineHeight: size * 0.7,
          fontWeight: "700",
        }}
      >
        {Array.from(name.trim())[0]?.toUpperCase() ?? "?"}
      </Text>
    </View>
  );
}

function ModelTags({
  run,
  theme,
  scale,
  compact = false,
}: {
  run: BoardRun;
  theme: PluginSurfaceProps["theme"];
  scale: number;
  /** Plain icon rows for subagent cards; full cards use bordered tags. */
  compact?: boolean;
}) {
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const tag = (icon: string, label: string, color: string) => (
    <View
      style={{
        flexDirection: "row",
        alignItems: "center",
        gap: s(4),
        minWidth: 0,
        flexShrink: 1,
        ...(compact
          ? {}
          : {
              paddingHorizontal: s(7),
              paddingVertical: s(2),
              borderWidth: 1,
              borderColor: color === colors.foreground ? colors.border : color,
              borderRadius: s(6),
              backgroundColor: colors.surface2,
            }),
      }}
    >
      <Icon name={icon} size={s(compact ? 12 : 14)} color={color} />
      <Text
        numberOfLines={1}
        style={{
          color,
          fontSize: s(compact ? 11.5 : 12.5),
          lineHeight: s(compact ? 15 : 17),
          fontWeight: "600",
          flexShrink: 1,
        }}
      >
        {label}
      </Text>
    </View>
  );
  return (
    <View
      style={{ flexDirection: "row", alignItems: "center", gap: s(compact ? 8 : 6), minWidth: 0 }}
    >
      {tag("Cpu", run.model ?? run.provider, colors.foreground)}
      {run.effort
        ? tag(
            "Brain",
            run.effort,
            effortColor(run.effort, colors.surface2) ?? colors.foregroundMuted,
          )
        : null}
    </View>
  );
}

/** Model and effort when known; the provider stands in until the model is read. */
function modelLabel(run: BoardRun) {
  return [run.model ?? run.provider, run.effort].filter(Boolean).join(" · ");
}

// Constant for the session; reading it per card render was wasted work.
const HOVER_ACTIONS =
  Platform.OS === "web" &&
  (globalThis as { matchMedia?: (query: string) => { matches: boolean } }).matchMedia?.(
    "(hover: hover) and (pointer: fine)",
  ).matches === true;

/** Keeps callback identity stable across renders while always calling the latest closure. */
function useStableCallback<Args extends unknown[], Result>(
  callback: (...args: Args) => Result,
): (...args: Args) => Result {
  const latest = useRef(callback);
  latest.current = callback;
  return useCallback((...args: Args) => latest.current(...args), []);
}

type CardPart = "card" | "repo" | "star" | "remove" | "diffs";

function RunCard({
  run,
  projectHue,
  theme,
  onRemove,
  onOpen,
  onToggleDiffs,
  onOpenProject,
  onStar,
  onFocusChange,
  scale,
  embedded = false,
  subagent = false,
  inheritedProject = false,
  removeCount = 1,
  seen = false,
  orb,
}: {
  projectHue?: number;
  /** Finished and opened since its last turn: neutral surface, content dimmed until hover. */
  seen?: boolean;
  /** Saved thinking-orb settings; null until loaded, which keeps the host spinner. */
  orb: OrbSettings | null;
  /** Rendered inside a cluster card: no own border. */
  embedded?: boolean;
  /** Compact two-line card inside the parent subagent panel. */
  subagent?: boolean;
  /** Subagents omit the project when the parent card already names the same one. */
  inheritedProject?: boolean;
  /** Conversations hidden by Remove, including this one and its subagents. */
  removeCount?: number;
  scale: number;
  run: BoardRun;
  theme: PluginSurfaceProps["theme"];
  onRemove?: (id: string) => Promise<void>;
  onOpen?: (agentId: string) => void;
  /** Turns edit diffs on or off for the thread without opening it. */
  onToggleDiffs?: (agentId: string, off: boolean) => Promise<void>;
  /** Starts a conversation in the card's project from its repo label. */
  onOpenProject?: (run: BoardRun) => void;
  onStar: (id: string, starred: boolean) => Promise<void>;
  /** Keyboard focus of the open action, for a cluster that draws the ring around all of it. */
  onFocusChange?: (focused: boolean) => void;
}) {
  const [hovered, setHovered] = useState(false);
  const [starHovered, setStarHovered] = useState(false);
  const [diffsHovered, setDiffsHovered] = useState(false);
  const [togglingDiffs, setTogglingDiffs] = useState(false);
  const [diffsError, setDiffsError] = useState(false);
  const [repoHovered, setRepoHovered] = useState(false);
  const [focus, setFocus] = useState<{ part: CardPart; keyboard: boolean } | null>(null);
  const [removeHovered, setRemoveHovered] = useState(false);
  const [starring, setStarring] = useState(false);
  const [starError, setStarError] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [removeError, setRemoveError] = useState(false);
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const running = run.status === "running";
  // Web shows running work as a thinking orb; native keeps the avatar and host spinner.
  const thinking = orbSupported && orb?.enabled && running && !run.needsInput ? orb : null;
  const duration = useClock((now) => runDuration(run, now));
  const tone = run.needsInput
    ? colors.statusWarning
    : running
      ? colors.accent
      : run.status === "failed"
        ? colors.statusDanger
        : run.status === "completed"
          ? colors.statusSuccess
          : colors.statusWarning;
  const relative = useClock((now) => relativeLabel(run.endedAt, now));
  const timing = running
    ? (duration ?? "Timing unavailable")
    : run.status === "unknown"
      ? relative === "Finish time unavailable"
        ? "Observation time unavailable"
        : `Observed ${relative.toLowerCase()}`
      : relative;
  const others = removeCount - 1;
  const removeLabel = removing
    ? "Removing…"
    : others > 0
      ? `Remove all · ${removeCount}`
      : "Remove";
  const removeHint = others > 0 ? ` and ${others} ${others === 1 ? "subagent" : "subagents"}` : "";
  const padX = s(subagent ? 8 : 12);
  const padY = s(subagent ? 10 : 16);
  const showStar = run.starred || hovered || starHovered || repoHovered || starring;
  const hoverActions = HOVER_ACTIONS;
  const actionFocused = focus?.part === "star" || focus?.part === "remove";
  const actionsVisible =
    !hoverActions ||
    showStar ||
    actionFocused ||
    removeHovered ||
    removing ||
    starError ||
    removeError;
  // The ring is for keyboard focus only; a click focuses the element without drawing it.
  const ring = (part: CardPart, offset?: number) =>
    focus?.part === part && focus.keyboard ? focusRing(colors.accent, offset) : null;
  const focusProps = (part: CardPart) => ({
    onFocus: (event: unknown) => {
      const keyboard = keyboardFocus(event);
      setFocus({ part, keyboard });
      if (part === "card") onFocusChange?.(keyboard);
    },
    onBlur: () => {
      setFocus(null);
      if (part === "card") onFocusChange?.(false);
    },
  });

  const starButton = (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${run.starred ? "Unstar" : "Star"} ${run.title}`}
      accessibilityState={{ selected: run.starred, disabled: starring }}
      disabled={starring}
      {...focusProps("star")}
      onHoverIn={() => setStarHovered(true)}
      onHoverOut={() => setStarHovered(false)}
      onPress={async () => {
        setStarring(true);
        setStarError(false);
        try {
          await onStar(run.id, !run.starred);
        } catch {
          setStarError(true);
        } finally {
          setStarring(false);
        }
      }}
      hitSlop={subagent && hoverActions ? 0 : s(8)}
      style={{
        width: s(subagent ? 24 : 28),
        height: s(subagent ? 24 : 28),
        // Keeps the repo row one text line tall.
        marginVertical: subagent ? 0 : -s(6),
        marginRight: subagent ? 0 : -s(6),
        alignItems: "center",
        justifyContent: "center",
        borderRadius: s(CONTROL_RADIUS),
        backgroundColor: starHovered && !starring ? colors.surface2 : "transparent",
        // Unstarred stars stay quiet until the card is hovered; touch clients keep them visible.
        opacity: starring ? 0.5 : subagent || showStar || actionFocused || !hoverActions ? 1 : 0.35,
        ...ring("star"),
      }}
    >
      {run.starred ? (
        <Text
          accessible={false}
          style={{
            color: colors.statusWarning,
            fontSize: s(subagent ? 16 : 20),
            lineHeight: s(subagent ? 20 : 24),
          }}
        >
          ★
        </Text>
      ) : (
        <Icon
          name="Star"
          size={s(subagent ? 13 : 16)}
          color={starHovered ? colors.statusWarning : colors.foregroundMuted}
        />
      )}
    </Pressable>
  );
  // Subagent rows have no repo row, so only top-level cards carry it.
  const diffsOff = Boolean(run.diffsOff);
  const diffsButton =
    onToggleDiffs && !subagent ? (
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`Turn edit diffs ${diffsOff ? "on" : "off"} for ${run.title}`}
        accessibilityState={{ selected: diffsOff, disabled: togglingDiffs }}
        disabled={togglingDiffs}
        {...focusProps("diffs")}
        onHoverIn={() => setDiffsHovered(true)}
        onHoverOut={() => setDiffsHovered(false)}
        onPress={async () => {
          setTogglingDiffs(true);
          setDiffsError(false);
          try {
            await onToggleDiffs(run.agentId, !diffsOff);
          } catch {
            setDiffsError(true);
          } finally {
            setTogglingDiffs(false);
          }
        }}
        hitSlop={s(8)}
        style={{
          width: s(28),
          height: s(28),
          marginVertical: -s(6),
          alignItems: "center",
          justifyContent: "center",
          borderRadius: s(CONTROL_RADIUS),
          backgroundColor: diffsHovered && !togglingDiffs ? colors.surface2 : "transparent",
          // Off stays visible like a star; on shows with the star on hover; touch clients keep it.
          opacity: togglingDiffs
            ? 0.5
            : diffsOff || showStar || diffsHovered || focus?.part === "diffs" || !hoverActions
              ? 1
              : 0,
          ...ring("diffs"),
        }}
      >
        <Icon
          name={diffsOff ? "PencilOff" : "Pencil"}
          size={s(16)}
          color={diffsHovered || diffsOff ? colors.foreground : colors.foregroundMuted}
        />
      </Pressable>
    ) : null;
  const removeButton = onRemove ? (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`Remove ${run.title}${removeHint} from Board`}
      disabled={removing}
      {...focusProps("remove")}
      onHoverIn={() => setRemoveHovered(true)}
      onHoverOut={() => setRemoveHovered(false)}
      onPress={async () => {
        setRemoving(true);
        setRemoveError(false);
        try {
          await onRemove(run.id);
        } catch {
          setRemoveError(true);
        } finally {
          setRemoving(false);
        }
      }}
      hitSlop={subagent ? 0 : s(6)}
      style={{
        paddingHorizontal: s(subagent ? 5 : 8),
        paddingVertical: s(subagent ? 5 : 4),
        marginVertical: subagent ? 0 : -s(4),
        marginRight: subagent ? 0 : -s(8),
        borderRadius: s(CONTROL_RADIUS),
        backgroundColor: removeHovered && !removing ? colors.surface2 : "transparent",
        opacity: removing ? 0.5 : 1,
        ...ring("remove"),
      }}
    >
      {subagent ? (
        <Icon name="X" size={s(14)} color={colors.foregroundMuted} />
      ) : (
        <Text
          style={{
            color: removeHovered && !removing ? colors.statusDanger : colors.foregroundMuted,
            fontSize: s(12),
            lineHeight: s(16),
            fontWeight: "600",
          }}
        >
          {removeLabel}
        </Text>
      )}
    </Pressable>
  ) : null;

  const label = run.needsInput ? "Needs input" : statusLabel(run.status);
  const active =
    hovered || starHovered || diffsHovered || repoHovered || removeHovered || focus?.keyboard;
  const dim = seen && !active ? SEEN_OPACITY : 1;
  const readLabel = running || subagent ? "" : seen ? ", seen" : run.unread ? ", new" : "";
  // Only attention and non-success outcomes take a status color; the rest stays muted.
  const labelColor =
    run.needsInput || (!running && run.status !== "completed") ? tone : colors.foregroundMuted;
  const metaStyle = {
    color: colors.foregroundMuted,
    fontSize: s(subagent ? 11.5 : 12),
    lineHeight: s(subagent ? 15 : 16),
    flex: 1,
    ...TABULAR,
  } satisfies TextStyle;

  return (
    <View
      accessibilityLabel={`${run.title}, ${label}${readLabel}`}
      style={{
        borderRadius: s(subagent ? CONTROL_RADIUS : embedded ? 0 : CARD_RADIUS),
        borderWidth: embedded || subagent ? 0 : 1,
        ...repoSurface(seen ? undefined : projectHue, theme),
        ...(subagent || embedded ? { backgroundColor: "transparent" } : {}),
        overflow: "hidden",
        // An embedded parent hands its ring to the cluster, whose clipping would hide it here.
        ...(subagent ? ring("card", -2) : embedded ? null : ring("card", 2)),
      }}
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`Open conversation ${run.title}`}
        accessibilityHint={`${run.project}, ${modelLabel(run)}, ${label}, ${timing}${duration ? `, duration ${duration}` : ""}`}
        disabled={!onOpen}
        onPress={() => onOpen?.(run.agentId)}
        {...focusProps("card")}
        onHoverIn={() => setHovered(true)}
        onHoverOut={() => setHovered(false)}
        style={({ pressed }) => ({
          position: "absolute",
          inset: 0,
          backgroundColor: pressed ? colors.surface2 : hovered ? colors.surface2 : "transparent",
          opacity: pressed ? 1 : hovered ? 0.55 : 1,
          ...NO_OUTLINE,
        })}
      />
      <View
        pointerEvents="box-none"
        style={{ paddingHorizontal: padX, paddingVertical: padY, gap: s(6) }}
      >
        {subagent ? (
          <View
            pointerEvents="box-none"
            style={{ flexDirection: "row", alignItems: "center", gap: s(8), minHeight: s(32) }}
          >
            {/* The elbow marks the row as nested under the card above. */}
            <View
              pointerEvents="none"
              accessibilityElementsHidden
              importantForAccessibility="no-hide-descendants"
              style={{ opacity: 0.6 * dim }}
            >
              <Icon name="CornerDownRight" size={s(14)} color={colors.foregroundMuted} />
            </View>
            <View pointerEvents="none" style={{ opacity: dim }}>
              <AgentAvatar agentId={run.agentId} size={s(24)} />
            </View>
            <View pointerEvents="none" style={{ flex: 1, minWidth: 0, gap: s(1), opacity: dim }}>
              <Text
                numberOfLines={1}
                style={{
                  color: colors.foreground,
                  fontSize: s(13),
                  lineHeight: s(17),
                  fontWeight: "600",
                }}
              >
                {run.title}
              </Text>
              <View style={{ flexDirection: "row", alignItems: "center", gap: s(4), minWidth: 0 }}>
                {run.needsInput ? (
                  <AttentionPulse grow={1.2}>
                    <Icon name="CircleAlert" size={s(12)} color={tone} />
                  </AttentionPulse>
                ) : thinking ? (
                  <Orb
                    size={s(12)}
                    state={thinking.state}
                    theme={theme}
                    color={colors.foregroundMuted}
                  />
                ) : running ? (
                  <Spinner color={colors.foregroundMuted} size={s(12)} />
                ) : (
                  <Icon
                    name={
                      run.status === "completed"
                        ? "CircleCheck"
                        : run.status === "failed"
                          ? "CircleX"
                          : "CircleHelp"
                    }
                    size={s(12)}
                    color={tone}
                  />
                )}
                <Text numberOfLines={1} style={metaStyle}>
                  <Text style={{ color: labelColor, fontWeight: "500" }}>{label}</Text>
                  {duration ? `${SEPARATOR}${duration}` : null}
                  {inheritedProject ? null : SEPARATOR}
                  {inheritedProject ? null : <Text>{run.project}</Text>}
                </Text>
              </View>
            </View>
            <View
              pointerEvents={actionsVisible ? "box-none" : "none"}
              style={{
                position: hoverActions ? "absolute" : "relative",
                right: -s(2),
                top: s(2),
                flexDirection: "row",
                borderRadius: s(CONTROL_RADIUS),
                backgroundColor: colors.surface1,
                opacity: actionsVisible ? 1 : 0,
              }}
            >
              {starButton}
              {running ? null : removeButton}
            </View>
          </View>
        ) : (
          <>
            {/* Repo row: the project in its own hue, with Star at the card's right edge. */}
            <View
              pointerEvents="box-none"
              style={{
                flexDirection: "row",
                alignItems: "center",
                justifyContent: "space-between",
                gap: s(8),
                marginBottom: s(2),
              }}
            >
              <Pressable
                accessibilityRole="button"
                accessibilityLabel={`New conversation in ${run.project}`}
                disabled={!onOpenProject}
                onPress={() => onOpenProject?.(run)}
                {...focusProps("repo")}
                onHoverIn={() => setRepoHovered(true)}
                onHoverOut={() => setRepoHovered(false)}
                hitSlop={s(6)}
                style={({ pressed }) => ({
                  flexShrink: 1,
                  minWidth: 0,
                  flexDirection: "row",
                  alignItems: "center",
                  gap: s(7),
                  paddingHorizontal: s(2),
                  borderRadius: s(CONTROL_RADIUS),
                  opacity: (pressed ? 0.6 : 1) * dim,
                  ...ring("repo"),
                })}
              >
                <ProjectMark
                  name={run.project}
                  color={repoMark(projectHue, theme)}
                  size={s(14)}
                  theme={theme}
                />
                <Text
                  numberOfLines={1}
                  style={{
                    color: repoLabel(projectHue, theme),
                    fontSize: s(12),
                    lineHeight: s(16),
                    fontWeight: "600",
                    flexShrink: 1,
                    textDecorationLine: repoHovered && onOpenProject ? "underline" : "none",
                  }}
                >
                  {run.project}
                </Text>
              </Pressable>
              <View pointerEvents="box-none" style={{ flexDirection: "row", alignItems: "center" }}>
                {diffsButton}
                {starButton}
              </View>
            </View>
            <View
              pointerEvents="none"
              style={{ flexDirection: "row", alignItems: "flex-start", gap: s(10), opacity: dim }}
            >
              {thinking ? (
                <OrbAvatar
                  agentId={run.agentId}
                  size={s(36)}
                  state={thinking.state}
                  opacity={thinking.avatarOpacity / 100}
                  theme={theme}
                />
              ) : (
                <AgentAvatar agentId={run.agentId} size={s(36)} />
              )}
              <View style={{ flex: 1, minWidth: 0, gap: s(3) }}>
                <Text
                  numberOfLines={2}
                  style={{
                    color: colors.foreground,
                    fontSize: s(14),
                    lineHeight: s(19),
                    fontWeight: "600",
                  }}
                >
                  {run.title}
                </Text>
                <View
                  style={{ minWidth: 0, flexDirection: "row", alignItems: "center", gap: s(6) }}
                >
                  {run.needsInput ? (
                    <AttentionPulse>
                      <Text
                        style={{
                          color: colors.surface1,
                          backgroundColor: colors.statusWarning,
                          fontSize: s(11.5),
                          lineHeight: s(15),
                          fontWeight: "700",
                          paddingHorizontal: s(7),
                          paddingVertical: s(1),
                          borderRadius: s(CONTROL_RADIUS),
                          overflow: "hidden",
                        }}
                      >
                        Needs input
                      </Text>
                    </AttentionPulse>
                  ) : thinking ? null : running ? (
                    <Spinner color={tone} size={s(14)} />
                  ) : (
                    <View
                      accessibilityElementsHidden
                      style={{
                        width: s(6),
                        height: s(6),
                        borderRadius: s(3),
                        backgroundColor: tone,
                      }}
                    />
                  )}
                  <Text numberOfLines={1} style={metaStyle}>
                    {running ? null : <Text style={{ color: labelColor }}>{label}</Text>}
                    {running ? null : SEPARATOR}
                    {timing}
                    {!running && duration ? `${SEPARATOR}ran ${duration}` : null}
                  </Text>
                </View>
              </View>
            </View>
          </>
        )}
        {/* Badges row: model and effort under the text column, Remove at the right edge. */}
        <View
          pointerEvents="box-none"
          style={{
            flexDirection: "row",
            alignItems: "center",
            gap: s(8),
            // Subagent badges clear the elbow and avatar; card badges clear the avatar.
            paddingLeft: s(subagent ? 54 : 46),
            marginTop: s(2),
            minHeight: subagent ? 0 : s(23),
          }}
        >
          <View
            pointerEvents="none"
            style={{ flex: 1, minWidth: 0, flexDirection: "row", overflow: "hidden", opacity: dim }}
          >
            <ModelTags run={run} theme={theme} scale={scale} compact={subagent} />
          </View>
          {subagent || running ? null : removeButton}
        </View>
        {starError ? (
          <Text
            pointerEvents="none"
            accessibilityRole="alert"
            style={{ color: colors.statusDanger, fontSize: s(12), lineHeight: s(16) }}
          >
            Could not update star. Please retry.
          </Text>
        ) : null}
        {diffsError ? (
          <Text
            pointerEvents="none"
            accessibilityRole="alert"
            style={{ color: colors.statusDanger, fontSize: s(12), lineHeight: s(16) }}
          >
            Could not change edit diffs. Please retry.
          </Text>
        ) : null}
        {removeError ? (
          <Text
            pointerEvents="none"
            accessibilityRole="alert"
            style={{ color: colors.statusDanger, fontSize: s(12), lineHeight: s(16) }}
          >
            Could not remove. Please retry.
          </Text>
        ) : null}
      </View>
      {run.needsInput ? (
        <AttentionPulse
          grow={1}
          style={{
            position: "absolute",
            inset: 0,
            borderWidth: 2,
            borderColor: colors.statusWarning,
            borderRadius: s(subagent ? CONTROL_RADIUS : embedded ? 0 : CARD_RADIUS - 1),
          }}
        />
      ) : null}
    </View>
  );
}

function clusterSummary(tree: RunTree): { running: number; needsInput: number } {
  const total = { running: 0, needsInput: 0 };
  const visit = (node: RunTree) => {
    if (node.run.status === "running") total.running += 1;
    if (node.run.needsInput) total.needsInput += 1;
    node.children.forEach(visit);
  };
  tree.children.forEach(visit);
  return total;
}

type ClusterProps = Omit<
  React.ComponentProps<typeof RunCard>,
  "run" | "embedded" | "subagent" | "removeCount" | "onFocusChange"
> & {
  tree: RunTree;
  compact: boolean;
  collapsed: ReadonlySet<string>;
  onToggle: (id: string) => void;
  depth?: number;
};

function RunCluster({ tree, collapsed, onToggle, depth = 0, ...card }: ClusterProps) {
  const { theme, scale } = card;
  const colors = theme.colors;
  const s = (value: number) => value * scale;
  const expanded = !collapsed.has(tree.run.agentId);
  const [toggleHovered, setToggleHovered] = useState(false);
  const [toggleFocused, setToggleFocused] = useState(false);
  const [rootFocused, setRootFocused] = useState(false);
  const [panelWidth, setPanelWidth] = useState(0);
  // Remove cascades to subagents, so it waits until the whole cluster has finished.
  const onRemove = tree.running ? undefined : card.onRemove;
  // The root's read state covers the whole cluster; subagents are read through their parent.
  const seen = depth === 0 ? !tree.running && tree.run.unread === false : card.seen;
  if (!tree.children.length)
    return (
      <RunCard {...card} run={tree.run} subagent={depth > 0} onRemove={onRemove} seen={seen} />
    );
  const summary = clusterSummary(tree);
  const summaryColor = summary.needsInput
    ? colors.statusWarning
    : summary.running
      ? colors.accent
      : null;
  const nested = depth > 0;
  // Measure this panel so nested clusters and Board zoom use their actual available width.
  const edge = s(6);
  const gap = s(2);
  // A lone subagent keeps the full row so its title is not cut for an empty second column.
  const twoColumns = tree.children.length > 1 && panelWidth >= s(406);
  return (
    // The parent card is the only surface; subagents are rows inside it, not boxes in boxes.
    <View
      style={
        nested
          ? null
          : {
              borderWidth: 1,
              ...repoSurface(seen ? undefined : card.projectHue, theme),
              borderRadius: s(CARD_RADIUS),
              overflow: "hidden",
              // One ring around the whole card when its open action has keyboard focus.
              ...(rootFocused ? focusRing(colors.accent, 2) : null),
            }
      }
    >
      <RunCard
        {...card}
        run={tree.run}
        embedded
        subagent={nested}
        onRemove={onRemove}
        removeCount={tree.count}
        seen={seen}
        onFocusChange={nested ? undefined : setRootFocused}
      />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`${expanded ? "Collapse" : "Expand"} subagents of ${tree.run.title}`}
        accessibilityState={{ expanded }}
        accessibilityHint={expanded ? "Hides the subagent cards" : "Shows the subagent cards"}
        onPress={() => onToggle(tree.run.agentId)}
        onFocus={(event) => setToggleFocused(keyboardFocus(event))}
        onBlur={() => setToggleFocused(false)}
        onHoverIn={() => setToggleHovered(true)}
        onHoverOut={() => setToggleHovered(false)}
        style={({ pressed }) => ({
          minHeight: 32,
          paddingVertical: s(6),
          paddingHorizontal: s(nested ? 6 : 12),
          flexDirection: "row",
          alignItems: "center",
          gap: s(8),
          borderTopWidth: nested ? 0 : 1,
          borderTopColor: repoDivider(seen ? undefined : card.projectHue, theme),
          borderRadius: nested ? s(CONTROL_RADIUS) : 0,
          backgroundColor: toggleHovered ? colors.surface2 : "transparent",
          opacity: pressed ? 0.7 : 1,
          ...(toggleFocused ? focusRing(colors.accent, -2) : null),
        })}
      >
        <View
          accessibilityElementsHidden
          importantForAccessibility="no-hide-descendants"
          style={{ width: s(12), alignItems: "center", justifyContent: "center" }}
        >
          <View
            style={{
              width: s(5),
              height: s(5),
              borderRightWidth: 1.5,
              borderBottomWidth: 1.5,
              borderColor: colors.foregroundMuted,
              transform: [{ rotate: expanded ? "45deg" : "-45deg" }],
            }}
          />
        </View>
        <Text
          style={{
            color: toggleHovered ? colors.foreground : colors.foregroundMuted,
            fontSize: s(12),
            lineHeight: s(16),
            fontWeight: "600",
            flexGrow: 1,
            ...TABULAR,
          }}
        >
          {tree.count - 1} {tree.count === 2 ? "subagent" : "subagents"}
        </Text>
        {/* Collapsed rows carry the member status; expanded members show their own. */}
        {!expanded && summaryColor ? (
          <View style={{ flexDirection: "row", alignItems: "center", gap: s(6) }}>
            {summary.needsInput ? (
              <AttentionPulse
                grow={1.5}
                style={{
                  width: s(6),
                  height: s(6),
                  borderRadius: s(3),
                  backgroundColor: summaryColor,
                }}
              />
            ) : (
              <Spinner color={summaryColor} size={s(12)} />
            )}
            <Text
              style={{
                color: summary.needsInput ? summaryColor : colors.foregroundMuted,
                fontSize: s(12),
                lineHeight: s(16),
                fontWeight: "500",
              }}
            >
              {summary.needsInput
                ? `${summary.needsInput} needs input`
                : `${summary.running} running`}
            </Text>
          </View>
        ) : null}
      </Pressable>
      {expanded ? (
        <View
          onLayout={({ nativeEvent }) => setPanelWidth(nativeEvent.layout.width)}
          style={{
            paddingHorizontal: edge,
            paddingBottom: nested ? 0 : edge,
            marginLeft: nested ? s(11) : 0,
            borderLeftWidth: nested ? 1 : 0,
            borderLeftColor: colors.border,
            columnGap: edge,
            rowGap: gap,
            flexDirection: "row",
            flexWrap: "wrap",
            alignItems: "flex-start",
          }}
        >
          {tree.children.map((child) => (
            <View
              key={child.run.id}
              style={{
                width: twoColumns ? (panelWidth - edge * 3 - (nested ? 1 : 0)) / 2 : "100%",
                minWidth: 0,
              }}
            >
              <RunCluster
                {...card}
                seen={seen}
                tree={child}
                collapsed={collapsed}
                onToggle={onToggle}
                depth={depth + 1}
                inheritedProject={child.run.projectKey === tree.run.projectKey}
              />
            </View>
          ))}
        </View>
      ) : null}
    </View>
  );
}

// Memoized: the Board rerenders on every poll, but columns change only when their runs do.
const RunColumn = memo(function RunColumn({
  orb,
  projectPalette,
  title,
  trees,
  compact,
  collapsed,
  onToggle,
  emptyMessage,
  theme,
  onRemove,
  onOpen,
  onToggleDiffs,
  onOpenProject,
  onStar,
  scale,
}: {
  orb: OrbSettings | null;
  scale: number;
  projectPalette: Record<string, number>;
  title: string;
  trees: RunTree[];
  compact: boolean;
  collapsed: ReadonlySet<string>;
  onToggle: (id: string) => void;
  emptyMessage: string;
  theme: PluginSurfaceProps["theme"];
  onRemove?: (id: string) => Promise<void>;
  onOpen?: (agentId: string) => void;
  onToggleDiffs?: (agentId: string, off: boolean) => Promise<void>;
  onOpenProject?: (run: BoardRun) => void;
  onStar: (id: string, starred: boolean) => Promise<void>;
}) {
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const runs = trees.map((tree) => ({ ...tree.run, starred: tree.starred }));
  const byId = new Map(trees.map((tree) => [tree.run.id, tree]));
  const count = trees.reduce((total, tree) => total + tree.count, 0);
  const unread = trees.filter((tree) => !tree.running && tree.run.unread).length;
  // Cards keep the project grouping order; each card names its own project.
  const cards = groupRuns(runs).projects.flatMap((project) => project.runs);
  const labelStyle = {
    color: colors.foregroundMuted,
    fontSize: s(12),
    lineHeight: s(16),
    fontWeight: "600",
  } satisfies TextStyle;
  return (
    <View
      accessibilityLabel={`${title}, ${count} conversations${unread ? `, ${unread} new` : ""}`}
      style={{ flex: 1, minWidth: 0, gap: s(10) }}
    >
      {/* Every host lane names its two status columns, with their counts. */}
      <View
        style={{
          flexDirection: "row",
          alignItems: "baseline",
          gap: s(6),
          paddingHorizontal: s(2),
          marginBottom: s(2),
        }}
      >
        <Text accessibilityRole="header" style={labelStyle}>
          {title}
        </Text>
        <Text style={{ ...labelStyle, fontWeight: "400", ...TABULAR }}>{count}</Text>
        {unread ? (
          <View style={{ flexDirection: "row", alignItems: "center", gap: s(5) }}>
            <View
              accessibilityElementsHidden
              style={{
                width: s(6),
                height: s(6),
                borderRadius: s(3),
                backgroundColor: colors.statusSuccess,
              }}
            />
            <Text style={{ ...labelStyle, color: colors.statusSuccess, ...TABULAR }}>
              {unread} new
            </Text>
          </View>
        ) : null}
      </View>
      {cards.length ? (
        cards.map((run) => (
          <RunCluster
            key={run.id}
            projectHue={run.projectId ? projectPalette[run.projectId] : undefined}
            tree={byId.get(run.id)!}
            compact={compact}
            collapsed={collapsed}
            onToggle={onToggle}
            scale={scale}
            theme={theme}
            onRemove={onRemove}
            onOpen={onOpen}
            onToggleDiffs={onToggleDiffs}
            onOpenProject={onOpenProject}
            onStar={onStar}
            orb={orb}
          />
        ))
      ) : (
        <Text
          style={{
            color: colors.foregroundMuted,
            fontSize: s(12),
            lineHeight: s(16),
            paddingHorizontal: s(2),
          }}
        >
          {emptyMessage}
        </Text>
      )}
    </View>
  );
});

/** Device glyph from the host name; names that say nothing get the generic machine glyph. */
function deviceIcon(label: string) {
  return /macbook|laptop|notebook/i.test(label) ? "Laptop" : "HardDrive";
}

/** Hosts usually report an mDNS name; the lane shows it without the suffix. */
function hostName(label: string) {
  return label.replace(/\.local$/i, "");
}

/** Host rail: device, name, connection, a hairline, then the lane's counts. */
function laneHeader({
  label,
  meta,
  figures,
  connection,
  tone,
  live = false,
  theme,
  scale,
}: {
  label: string;
  /** Plain summary shown when the lane has no figures. */
  meta: string | null;
  figures: { running: number; finished: number } | null;
  connection: string;
  /** Connection status color: the dot always carries it, the text only when not live. */
  tone: string;
  live?: boolean;
  theme: PluginSurfaceProps["theme"];
  scale: number;
}) {
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const metaStyle = {
    color: colors.foregroundMuted,
    fontSize: s(12),
    lineHeight: s(16),
    ...TABULAR,
  } satisfies TextStyle;
  return (
    // Opaque, so cards do not show through the rail while it sticks.
    <View
      style={[
        {
          zIndex: 1,
          flexDirection: "row",
          alignItems: "center",
          gap: s(10),
          height: s(RAIL_HEIGHT),
          backgroundColor: colors.surface0,
        },
        STICKY,
      ]}
    >
      <View accessibilityElementsHidden importantForAccessibility="no-hide-descendants">
        <Icon name={deviceIcon(label)} size={s(18)} color={colors.foreground} />
      </View>
      <Text
        accessibilityRole="header"
        numberOfLines={1}
        style={{
          color: colors.foreground,
          fontSize: s(15),
          lineHeight: s(20),
          fontWeight: "600",
          letterSpacing: s(-0.15),
          flexShrink: 1,
        }}
      >
        {hostName(label)}
      </Text>
      <View style={{ flexDirection: "row", alignItems: "center", gap: s(6) }}>
        <View
          accessibilityElementsHidden
          importantForAccessibility="no-hide-descendants"
          style={{ width: s(6), height: s(6), borderRadius: s(3), backgroundColor: tone }}
        />
        <Text
          accessibilityLabel={`Connection ${connection}`}
          numberOfLines={1}
          style={{ ...metaStyle, color: live ? colors.foregroundMuted : tone }}
        >
          {connection}
        </Text>
      </View>
      <View
        accessibilityElementsHidden
        importantForAccessibility="no-hide-descendants"
        style={{ flex: 1, minWidth: s(12), height: 1, backgroundColor: colors.border }}
      />
      {figures ? (
        <Text
          accessibilityLabel={`${figures.running} running, ${figures.finished} finished`}
          numberOfLines={1}
          style={metaStyle}
        >
          {`${figures.running} running${SEPARATOR}${figures.finished} finished`}
        </Text>
      ) : meta ? (
        <Text numberOfLines={1} style={{ ...metaStyle, flexShrink: 1 }}>
          {meta}
        </Text>
      ) : null}
    </View>
  );
}

/** One host lane: its rail, then the lane body. */
function lane(scale: number, header: ReactNode, children: ReactNode) {
  return (
    <View style={{ flexShrink: 0, minWidth: 0 }}>
      {header}
      <View style={{ minWidth: 0, gap: scale * 10, marginTop: scale * 4 }}>{children}</View>
    </View>
  );
}

const NO_RUNS: BoardRun[] = [];
const NO_HUES: Record<string, number> = {};

function HostBoard({
  host,
  theme,
  layout,
  navigation,
  client,
  status,
  scale,
  view,
}: PluginSurfaceProps & {
  client: BoardHostClient;
  status: PluginHostSummary["status"];
  scale: number;
  view: "runs" | "recaps";
}) {
  const online = status === "online";
  const palette = useHostSettings(projectColors, host.id, client, online);
  const savingPalette = useRef(false);
  const orbConfig = useHostSettings(orbSettings, host.id, client, online);
  const orb = orbConfig.values ?? null;
  const readBoard = () => client.rpc(boardRpc, {});
  const removeRun = (input: { id: string; observingSince: string; endedAt: string | null }) =>
    client.rpc(removeRunRpc, input);
  const setStarred = (input: { id: string; observingSince: string; starred: boolean }) =>
    client.rpc(starRunRpc, input);
  const board = useQuery({
    queryKey: ["board", host.id],
    queryFn: readBoard,
    enabled: online,
    retry: false,
    refetchInterval: (query) => boardPollInterval(query.state.data),
    refetchOnWindowFocus: false,
    // Opening a thread unmounts Board. Drop its cached snapshot so a thread's Remove
    // action cannot briefly show the removed card when Board mounts again.
    gcTime: 0,
  });
  const onStar = useStableCallback(async (id: string, starred: boolean) => {
    if (!online) throw new Error("Host is offline. Reconnect and retry.");
    const result = await setStarred({ id, starred, observingSince: board.data!.observingSince });
    if (!result.updated) throw new Error("Run changed. Refresh and retry.");
    await board.refetch({ throwOnError: true });
  });
  const runs = board.data?.runs ?? NO_RUNS;
  useEffect(() => {
    if (
      !online ||
      palette.status !== "ready" ||
      palette.saving ||
      palette.saveError ||
      savingPalette.current
    )
      return;
    const next = allocateColors(
      palette.values!.hues,
      runs.flatMap((run) => (run.projectId ? [run.projectId] : [])),
    );
    if (next === palette.values!.hues) return;
    savingPalette.current = true;
    void palette.save({ hues: next }, palette.revision).finally(() => {
      savingPalette.current = false;
    });
  }, [palette, board.dataUpdatedAt]);
  const projectPalette = palette.values?.hues ?? NO_HUES;

  const { running, finished } = useMemo(() => boardColumns(runs), [runs]);
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set());
  const onToggle = useCallback(
    (id: string) =>
      setCollapsed((previous) => {
        const next = new Set(previous);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      }),
    [],
  );
  // Same flow as the sidebar project "+" button: the New workspace screen with this project selected.
  const [projectError, setProjectError] = useState<string | null>(null);
  const onOpenProject = useStableCallback((run: BoardRun) => {
    setProjectError(null);
    try {
      if (!run.cwd) throw new Error("Project directory is unavailable.");
      const input = {
        serverId: host.id,
        cwd: run.cwd,
        name: run.project,
        projectId: run.projectId,
      };
      client.openNewWorkspace(input);
    } catch (error) {
      setProjectError(error instanceof Error ? error.message : String(error));
    }
  });
  const openAgentStable = useStableCallback((agentId: string) => {
    navigation?.openAgent({ agentId, serverId: host.id, anchor: "latest-prompt" });
  });
  const openAgent = navigation ? openAgentStable : undefined;
  const toggleDiffsStable = useStableCallback(async (agentId: string, off: boolean) => {
    await client.rpc(editDiffsModeRpc, { agentId, off });
    await board.refetch({ throwOnError: true });
  });
  const toggleDiffs = online ? toggleDiffsStable : undefined;
  const onRemove = useStableCallback(async (id: string) => {
    const run = runs.find((item) => item.id === id);
    if (!run) return;
    const removed = await removeFinishedRun(
      run,
      board.data!.observingSince,
      (scope) => removeRun({ id, observingSince: scope, endedAt: run.endedAt }),
      readBoard,
    );
    if (!removed) throw new Error("Run changed. Refresh and retry.");
    // The card is gone once the Board refreshes, so a failed close reports on the page.
    void closeRemovedRuntimes(client, run.agentId).catch((error: unknown) =>
      setProjectError(error instanceof Error ? error.message : String(error)),
    );
    await board.refetch({ throwOnError: true });
  });
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const connection = useClock((now) =>
    boardConnectionState({
      hasData: Boolean(board.data),
      isError: board.isError,
      isPaused: !online || board.isPaused,
      dataUpdatedAt: board.dataUpdatedAt,
      now,
    }),
  );
  const initialLoading = connection === "connecting";
  const unavailable = !board.data && (connection === "error" || connection === "offline");
  const connectionColor =
    connection === "error"
      ? colors.statusDanger
      : connection === "live"
        ? colors.statusSuccess
        : colors.statusWarning;
  const connectionLabel =
    connection === "error"
      ? "Connection issue"
      : connection === "offline"
        ? "Offline"
        : connection === "stale"
          ? "Stale"
          : connection === "connecting"
            ? "Connecting"
            : "Live";
  const snapshotWarning =
    connection === "error"
      ? "Could not refresh. Showing the last successful snapshot."
      : connection === "offline"
        ? "Host is offline. Showing the last successful snapshot."
        : connection === "stale"
          ? "Updates are delayed. Showing a snapshot older than 10 seconds."
          : null;

  const total = (trees: RunTree[]) => trees.reduce((sum, tree) => sum + tree.count, 0);
  const retryLabel = board.isFetching ? "Retrying…" : "Retry";
  const [retryFocused, setRetryFocused] = useState(false);
  // The rail already carries the connection color; the notice stays neutral.
  const notice = (message: string) => (
    <View
      accessibilityRole="alert"
      style={{
        flexDirection: "row",
        alignItems: "center",
        gap: s(10),
        paddingVertical: s(4),
        paddingLeft: s(10),
        paddingRight: s(4),
        borderRadius: s(CARD_RADIUS),
        backgroundColor: colors.surface2,
      }}
    >
      <Text
        style={{ flex: 1, color: colors.foregroundMuted, fontSize: s(12.5), lineHeight: s(17) }}
      >
        {message}
      </Text>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Retry loading Board"
        disabled={board.isFetching}
        onPress={() => void board.refetch()}
        onFocus={(event) => setRetryFocused(keyboardFocus(event))}
        onBlur={() => setRetryFocused(false)}
        hitSlop={s(8)}
        style={({ pressed }) => ({
          paddingHorizontal: s(10),
          paddingVertical: s(4),
          alignItems: "center",
          borderRadius: s(CONTROL_RADIUS),
          borderWidth: 1,
          borderColor: colors.border,
          backgroundColor: colors.surface0,
          opacity: pressed ? 0.7 : 1,
          ...(retryFocused ? focusRing(colors.accent) : null),
        })}
      >
        <Text
          style={{
            color: colors.foreground,
            fontSize: s(12.5),
            lineHeight: s(17),
            fontWeight: "600",
          }}
        >
          {retryLabel}
        </Text>
      </Pressable>
    </View>
  );
  return lane(
    scale,
    laneHeader({
      label: host.label,
      meta: view === "recaps" ? "Last 7 days" : null,
      figures:
        view === "runs" && board.data
          ? { running: total(running), finished: total(finished) }
          : null,
      connection: connectionLabel,
      tone: connectionColor,
      live: connection === "live",
      theme,
      scale,
    }),
    <>
      {palette.status === "error" || palette.saveError ? (
        <Pressable accessibilityRole="button" onPress={() => void palette.reload()}>
          <Text style={{ color: colors.statusWarning, fontSize: s(12.5), lineHeight: s(17) }}>
            Could not save project colors. Tap to retry.
          </Text>
        </Pressable>
      ) : null}
      {view === "recaps" ? (
        <RecapsView
          hostId={host.id}
          rpc={client.rpc}
          online={online}
          theme={theme}
          scale={scale}
          hues={projectPalette}
          onOpen={openAgent}
        />
      ) : initialLoading ? (
        <View
          style={{
            minHeight: s(72),
            flexDirection: "row",
            alignItems: "center",
            justifyContent: "center",
            gap: s(10),
          }}
        >
          <ActivityIndicator color={colors.accent} />
          <Text style={{ color: colors.foregroundMuted, fontSize: s(13), lineHeight: s(18) }}>
            Loading runs…
          </Text>
        </View>
      ) : unavailable ? (
        notice(
          connection === "offline"
            ? "Board is offline. Reconnect to the host, then retry."
            : "Board is unavailable. Check the host connection, then retry.",
        )
      ) : (
        <>
          {projectError ? (
            <Text
              accessibilityRole="alert"
              style={{ color: colors.statusDanger, fontSize: s(12.5), lineHeight: s(17) }}
            >
              {projectError}
            </Text>
          ) : null}
          {snapshotWarning ? notice(snapshotWarning) : null}
          <View
            style={{
              flexDirection: layout.compact ? "column" : "row",
              alignItems: "stretch",
              gap: s(layout.compact ? 16 : COLUMN_GAP),
            }}
          >
            <RunColumn
              orb={orb}
              projectPalette={projectPalette}
              scale={scale}
              title="Running"
              trees={running}
              compact={layout.compact}
              collapsed={collapsed}
              onToggle={onToggle}
              onRemove={online ? onRemove : undefined}
              onStar={onStar}
              emptyMessage="No conversations are running."
              theme={theme}
              onOpen={openAgent}
              onToggleDiffs={toggleDiffs}
              onOpenProject={onOpenProject}
            />
            <RunColumn
              orb={orb}
              projectPalette={projectPalette}
              scale={scale}
              title="Just finished"
              trees={finished}
              compact={layout.compact}
              collapsed={collapsed}
              onToggle={onToggle}
              onStar={onStar}
              emptyMessage="No finished conversations observed yet."
              theme={theme}
              onOpen={openAgent}
              onToggleDiffs={toggleDiffs}
              onOpenProject={onOpenProject}
              onRemove={online ? onRemove : undefined}
            />
          </View>
        </>
      )}
    </>,
  );
}

export function BoardPage(props: PluginSurfaceProps & { client: BoardHostClient }) {
  const { theme, layout } = props;
  const hosts = useHosts();
  const clients = useMemo(
    () =>
      new Map(
        hosts.map((target) => [
          target.serverId,
          createBoardHostClient(props.client, target.serverId),
        ]),
      ),
    [hosts, props.client],
  );
  // Host settings keep the size across plugin reloads and restarts without changing host appearance.
  const sizeSettings = useSettings(boardSize);
  const [view, setView] = useState<"runs" | "recaps">("runs");
  const savingSize = useRef(false);
  const failedSize = useRef<number | null>(null);
  // Latest unsaved choice; saved one write at a time so rapid clicks never reuse a stale revision.
  const [pendingSize, setPendingSize] = useState<number | null>(null);
  if (sizeSettings.status === "ready") lastSettings.size = sizeSettings.values.size;
  const storedSize =
    sizeSettings.status === "ready" ? sizeSettings.values.size : (lastSettings.size ?? null);
  const size = pendingSize ?? storedSize ?? BOARD_SIZE_DEFAULT;
  const scale = boardScale(size);
  const changeSize = (value: number) => {
    failedSize.current = null;
    setPendingSize(clampBoardSize(value));
  };
  useEffect(() => {
    if (pendingSize === null || sizeSettings.status !== "ready") return;
    if (sizeSettings.saving || savingSize.current) return;
    if (pendingSize === storedSize) return setPendingSize(null);
    // Retry a failed value only after the user picks a size again.
    if (failedSize.current === pendingSize) return;
    savingSize.current = true;
    void sizeSettings
      .save({ size: pendingSize }, sizeSettings.revision)
      .then((saved) => {
        failedSize.current = saved ? null : pendingSize;
      })
      .finally(() => {
        savingSize.current = false;
      });
  }, [pendingSize, sizeSettings]);

  const colors = theme.colors;
  const s = (value: number) => value * scale;
  const gutter = layout.compact ? 12 : 24;
  return (
    <View style={{ flex: 1, backgroundColor: colors.surface0 }}>
      <View
        style={{
          flexDirection: "row",
          flexWrap: "wrap",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 12,
          paddingHorizontal: gutter,
          paddingTop: 12,
          paddingBottom: 6,
        }}
      >
        <BoardViewSwitch view={view} onChange={setView} theme={theme} />
        <BoardSizeControl size={size} onChange={changeSize} theme={theme} />
      </View>
      <ScrollView
        style={{ flex: 1 }}
        contentContainerStyle={{ paddingHorizontal: gutter, paddingBottom: 40, flexGrow: 1 }}
      >
        <View
          style={{
            flexDirection: "column",
            alignItems: "stretch",
            gap: s(layout.compact ? 24 : 28),
            paddingTop: s(8),
          }}
        >
          {hosts.map((target) => {
            const client = clients.get(target.serverId);
            return client ? (
              <HostBoard
                {...props}
                key={target.serverId}
                client={client}
                host={{ id: target.serverId, label: target.label }}
                layout={layout}
                status={target.status}
                scale={scale}
                view={view}
              />
            ) : (
              <View key={target.serverId}>
                {lane(
                  scale,
                  laneHeader({
                    label: target.label,
                    meta:
                      target.status === "online"
                        ? "Board unavailable. Update or reload Board on this host."
                        : "Host offline. Reconnect to see its Board.",
                    figures: null,
                    connection: target.status === "online" ? "Unavailable" : "Offline",
                    tone: colors.statusWarning,
                    theme,
                    scale,
                  }),
                  null,
                )}
              </View>
            );
          })}
          {view === "runs" ? (
            <Text style={{ color: colors.foregroundMuted, fontSize: s(12), lineHeight: s(16) }}>
              Each host keeps its last 50 finished conversations.
            </Text>
          ) : null}
        </View>
      </ScrollView>
    </View>
  );
}
