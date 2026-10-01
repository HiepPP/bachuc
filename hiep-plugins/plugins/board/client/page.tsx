import type { PluginSurfaceProps } from "@getpaseo/plugin/client";
import { useRpc, useSettings } from "@getpaseo/plugin/client";
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
  type ViewStyle,
} from "react-native";
import { boardRpc, removeRunRpc, starRunRpc, groupRuns, type BoardRun } from "../shared/board";
import { boardColumns, type RunTree } from "../shared/tree";
import { BoardSizeControl } from "./size-control";
import { BoardViewSwitch, RecapsView } from "./recaps";
import { BOARD_SIZE_DEFAULT, boardScale, boardSize, clampBoardSize } from "../shared/board-size";
import { allocateColors, projectColors } from "../shared/project-colors";
import { boardConnectionState } from "./connection";
import { workspaceActions } from "./workspace";
import { AgentAvatar } from "./avatar";
import { Orb, OrbAvatar, orbSupported } from "./orb";
import { orbSettings, type OrbSettings } from "../shared/orb";
import { effortColor } from "../shared/effort";
import { lastSettings } from "./warm";
import { useClock } from "./clock";
import { removeFinishedRun } from "./remove";

const SECOND = 1_000;
// Shape lock: cards 12, controls and chips 8.
const CARD_RADIUS = 12;
const CONTROL_RADIUS = 8;

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
              borderRadius: s(CONTROL_RADIUS - 2),
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

function RunCard({
  run,
  theme,
  onRemove,
  onOpen,
  onStar,
  scale,
  embedded = false,
  subagent = false,
  inheritedProject = false,
  removeCount = 1,
  orb,
}: {
  /** Saved thinking-orb settings; null until loaded, which keeps the host spinner. */
  orb: OrbSettings | null;
  /** Rendered inside a cluster card: no own border. */
  embedded?: boolean;
  /** Compact two-line card inside the parent subagent panel. */
  subagent?: boolean;
  /** Hide the project row when the parent card already shows the same project. */
  inheritedProject?: boolean;
  /** Conversations hidden by Remove, including this one and its subagents. */
  removeCount?: number;
  scale: number;
  run: BoardRun;
  theme: PluginSurfaceProps["theme"];
  onRemove?: (id: string) => Promise<void>;
  onOpen?: (agentId: string) => void;
  onStar: (id: string, starred: boolean) => Promise<void>;
}) {
  const [hovered, setHovered] = useState(false);
  const [starHovered, setStarHovered] = useState(false);
  const [actionFocused, setActionFocused] = useState(false);
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
  const padX = s(subagent ? 8 : 16);
  const padY = s(subagent ? 8 : 14);
  const showStar = run.starred || hovered || starHovered || starring;
  const hoverActions = HOVER_ACTIONS;
  const actionsVisible =
    !hoverActions ||
    showStar ||
    actionFocused ||
    removeHovered ||
    removing ||
    starError ||
    removeError;

  const starButton = (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${run.starred ? "Unstar" : "Star"} ${run.title}`}
      accessibilityState={{ selected: run.starred, disabled: starring }}
      disabled={starring}
      onFocus={() => setActionFocused(true)}
      onBlur={() => setActionFocused(false)}
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
        marginTop: subagent ? 0 : -s(4),
        marginRight: subagent ? 0 : -s(6),
        alignItems: "center",
        justifyContent: "center",
        borderRadius: s(CONTROL_RADIUS),
        backgroundColor: starHovered && !starring ? colors.surface2 : "transparent",
        // Unstarred stars stay quiet until the card is hovered; touch clients keep them visible.
        opacity: starring ? 0.5 : subagent || showStar || actionFocused || !hoverActions ? 1 : 0.35,
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
  const removeButton = onRemove ? (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`Remove ${run.title}${removeHint} from Board`}
      disabled={removing}
      onFocus={() => setActionFocused(true)}
      onBlur={() => setActionFocused(false)}
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
        borderRadius: s(CONTROL_RADIUS - 2),
        backgroundColor: removeHovered && !removing ? colors.surface2 : "transparent",
        opacity: removing ? 0.5 : 1,
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

  return (
    <View
      accessibilityLabel={`${run.title}, ${run.needsInput ? "Needs input" : statusLabel(run.status)}`}
      style={{
        borderRadius: embedded ? 0 : s(subagent ? CARD_RADIUS - 2 : CARD_RADIUS),
        borderWidth: embedded ? 0 : 1,
        borderColor: subagent && run.needsInput ? colors.statusWarning : colors.border,
        backgroundColor: colors.surface1,
        overflow: "hidden",
      }}
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`Open conversation ${run.title}`}
        accessibilityHint={`${run.project}, ${modelLabel(run)}, ${run.needsInput ? "Needs input" : statusLabel(run.status)}, ${timing}${duration ? `, duration ${duration}` : ""}`}
        disabled={!onOpen}
        onPress={() => onOpen?.(run.agentId)}
        onHoverIn={() => setHovered(true)}
        onHoverOut={() => setHovered(false)}
        style={({ pressed }) => ({
          position: "absolute",
          inset: 0,
          backgroundColor: pressed ? colors.surface2 : hovered ? colors.surface2 : "transparent",
          opacity: pressed ? 1 : hovered ? 0.55 : 1,
        })}
      />
      <View
        pointerEvents="box-none"
        style={{
          paddingHorizontal: padX,
          paddingVertical: padY,
          gap: s(subagent ? 6 : 8),
        }}
      >
        {subagent ? (
          <View
            pointerEvents="box-none"
            style={{ flexDirection: "row", alignItems: "center", gap: s(8), minHeight: s(42) }}
          >
            <View pointerEvents="none">
              <AgentAvatar agentId={run.agentId} size={s(32)} />
            </View>
            <View pointerEvents="none" style={{ flex: 1, minWidth: 0, gap: s(3) }}>
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
              {inheritedProject ? null : (
                <Text
                  numberOfLines={1}
                  style={{ color: colors.foregroundMuted, fontSize: s(11), lineHeight: s(14) }}
                >
                  {run.project}
                </Text>
              )}
              <ModelTags run={run} theme={theme} scale={scale} compact />
              <View style={{ flexDirection: "row", alignItems: "center", gap: s(3), minWidth: 0 }}>
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
                <Text
                  numberOfLines={1}
                  style={{
                    color: running && !run.needsInput ? colors.foregroundMuted : tone,
                    fontSize: s(11),
                    lineHeight: s(15),
                    fontWeight: "500",
                    flexShrink: 1,
                  }}
                >
                  {run.needsInput ? "Needs input" : statusLabel(run.status)}
                  {duration ? (
                    <Text style={{ color: colors.foregroundMuted, fontWeight: "400" }}>
                      {` · ${duration}`}
                    </Text>
                  ) : null}
                </Text>
              </View>
            </View>
            <View
              pointerEvents={actionsVisible ? "box-none" : "none"}
              style={{
                position: hoverActions ? "absolute" : "relative",
                right: -s(4),
                top: -s(4),
                flexDirection: "row",
                borderRadius: s(6),
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
            <View
              pointerEvents="box-none"
              style={{ flexDirection: "row", alignItems: "flex-start", gap: s(10) }}
            >
              <View pointerEvents="none">
                {thinking ? (
                  <OrbAvatar
                    agentId={run.agentId}
                    size={s(40)}
                    state={thinking.state}
                    opacity={thinking.avatarOpacity / 100}
                    theme={theme}
                  />
                ) : (
                  <AgentAvatar agentId={run.agentId} size={s(40)} />
                )}
              </View>
              <View pointerEvents="none" style={{ flex: 1, gap: s(6) }}>
                <Text
                  numberOfLines={2}
                  style={{
                    color: colors.foreground,
                    fontSize: s(15),
                    lineHeight: s(21),
                    fontWeight: "600",
                  }}
                >
                  {run.title}
                </Text>
                <View
                  style={{
                    flexDirection: "row",
                    alignItems: "center",
                    gap: s(6),
                    flexWrap: "wrap",
                  }}
                >
                  {inheritedProject ? null : (
                    <Text
                      numberOfLines={1}
                      style={{
                        color: colors.foregroundMuted,
                        fontSize: s(12),
                        lineHeight: s(16),
                        fontWeight: "500",
                        flexShrink: 1,
                      }}
                    >
                      {run.project}
                    </Text>
                  )}
                  <ModelTags run={run} theme={theme} scale={scale} />
                </View>
              </View>
              {starButton}
            </View>
            <View
              pointerEvents="box-none"
              style={{
                flexDirection: "row",
                alignItems: "center",
                justifyContent: "space-between",
                flexWrap: "wrap",
                gap: s(8),
              }}
            >
              <View
                pointerEvents="none"
                style={{ flexDirection: "row", alignItems: "center", gap: s(7), flexShrink: 1 }}
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
                        paddingVertical: s(2),
                        borderRadius: s(CONTROL_RADIUS - 2),
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
                    style={{ width: s(7), height: s(7), borderRadius: s(4), backgroundColor: tone }}
                  />
                )}
                <Text
                  numberOfLines={1}
                  style={{
                    color: run.needsInput ? colors.foregroundMuted : tone,
                    fontSize: s(12.5),
                    lineHeight: s(17),
                    fontWeight: "600",
                    flexShrink: 1,
                  }}
                >
                  {run.needsInput ? timing : statusLabel(run.status)}
                  {run.needsInput ? null : (
                    <Text style={{ color: colors.foregroundMuted, fontWeight: "400" }}>
                      {"  "}
                      {timing}
                    </Text>
                  )}
                </Text>
              </View>
              {running ? null : (
                <View
                  pointerEvents="box-none"
                  style={{ flexDirection: "row", alignItems: "center", gap: s(10) }}
                >
                  <Text
                    style={{ color: colors.foregroundMuted, fontSize: s(12), lineHeight: s(16) }}
                  >
                    {duration ? `Ran ${duration}` : "Duration unavailable"}
                  </Text>
                  {removeButton}
                </View>
              )}
            </View>
          </>
        )}
        {starError ? (
          <Text
            pointerEvents="none"
            accessibilityRole="alert"
            style={{ color: colors.statusDanger, fontSize: s(12), lineHeight: s(16) }}
          >
            Could not update star. Please retry.
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
            borderRadius: embedded ? 0 : s(subagent ? CARD_RADIUS - 3 : CARD_RADIUS - 1),
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
  "run" | "embedded" | "subagent" | "removeCount"
> & {
  tree: RunTree;
  compact: boolean;
  collapsed: ReadonlySet<string>;
  onToggle: (id: string) => void;
  depth?: number;
};

function RunCluster({ tree, compact, collapsed, onToggle, depth = 0, ...card }: ClusterProps) {
  const { theme, scale } = card;
  const colors = theme.colors;
  const s = (value: number) => value * scale;
  const expanded = !collapsed.has(tree.run.agentId);
  const [toggleHovered, setToggleHovered] = useState(false);
  const [panelWidth, setPanelWidth] = useState(0);
  // Remove cascades to subagents, so it waits until the whole cluster has finished.
  const onRemove = tree.running ? undefined : card.onRemove;
  if (!tree.children.length)
    return <RunCard {...card} run={tree.run} subagent={depth > 0} onRemove={onRemove} />;
  const summary = clusterSummary(tree);
  const summaryColor = summary.needsInput
    ? colors.statusWarning
    : summary.running
      ? colors.accent
      : null;
  // Measure this panel so nested clusters and Board zoom use their actual available width.
  const edge = s(depth > 0 || compact ? 6 : 8);
  const pad = s(8);
  const gap = s(6);
  const twoColumns = panelWidth >= s(406);
  return (
    <View
      style={{
        borderWidth: 1,
        borderColor: colors.border,
        borderRadius: s(depth > 0 ? CARD_RADIUS - 2 : CARD_RADIUS),
        backgroundColor: colors.surface1,
        overflow: "hidden",
      }}
    >
      <RunCard
        {...card}
        run={tree.run}
        embedded
        subagent={depth > 0}
        onRemove={onRemove}
        removeCount={tree.count}
      />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`${expanded ? "Collapse" : "Expand"} subagents of ${tree.run.title}`}
        accessibilityState={{ expanded }}
        accessibilityHint={expanded ? "Hides the subagent cards" : "Shows the subagent cards"}
        onPress={() => onToggle(tree.run.agentId)}
        onHoverIn={() => setToggleHovered(true)}
        onHoverOut={() => setToggleHovered(false)}
        style={({ pressed }) => ({
          minHeight: 36,
          paddingVertical: s(7),
          paddingHorizontal: edge,
          flexDirection: "row",
          alignItems: "center",
          gap: s(8),
          borderTopWidth: 1,
          borderTopColor: colors.border,
          backgroundColor: toggleHovered ? colors.surface2 : "transparent",
          opacity: pressed ? 0.7 : 1,
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
            fontSize: s(12.5),
            lineHeight: s(17),
            fontWeight: "600",
            flexGrow: 1,
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
                color: summaryColor,
                fontSize: s(12),
                lineHeight: s(16),
                fontWeight: "600",
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
            marginHorizontal: edge,
            marginBottom: edge,
            padding: pad,
            gap,
            flexDirection: "row",
            flexWrap: "wrap",
            alignItems: "flex-start",
            borderRadius: s(CARD_RADIUS - 2),
            borderWidth: 1,
            borderColor: colors.border,
            backgroundColor: colors.surface2,
          }}
        >
          {tree.children.map((child) => (
            <View
              key={child.run.id}
              style={{
                width: twoColumns ? (panelWidth - pad * 2 - gap - 2) / 2 : "100%",
                minWidth: 0,
              }}
            >
              <RunCluster
                {...card}
                tree={child}
                compact={compact}
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
  onOpenProject?: (run: BoardRun) => void;
  onStar: (id: string, starred: boolean) => Promise<void>;
}) {
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const runs = trees.map((tree) => ({ ...tree.run, starred: tree.starred }));
  const byId = new Map(trees.map((tree) => [tree.run.id, tree]));
  const count = trees.reduce((total, tree) => total + tree.count, 0);
  const sections = groupRuns(runs);
  const renderCard = (run: BoardRun) => (
    <RunCluster
      key={run.id}
      tree={byId.get(run.id)!}
      compact={compact}
      collapsed={collapsed}
      onToggle={onToggle}
      scale={scale}
      theme={theme}
      onRemove={onRemove}
      onOpen={onOpen}
      onStar={onStar}
      orb={orb}
      // The project group heading already names the project.
      inheritedProject
    />
  );
  const hueOf = (project: { runs: BoardRun[] }) => {
    const id = project.runs[0].projectId;
    return id === undefined ? undefined : projectPalette[id];
  };
  const tint = (project: { runs: BoardRun[] }) => {
    const hue = hueOf(project);
    return hue === undefined ? colors.border : `hsl(${hue}, 60%, 72%)`;
  };
  const mark = (project: { runs: BoardRun[] }) => {
    const hue = hueOf(project);
    return hue === undefined ? colors.foregroundMuted : `hsl(${hue}, 42%, 58%)`;
  };
  const groupCount = (project: { runs: BoardRun[] }) =>
    project.runs.reduce((total, run) => total + byId.get(run.id)!.count, 0);
  const heading = (label: string, meta: string | null, color = colors.foregroundMuted) => (
    <View
      style={{ flexDirection: "row", alignItems: "baseline", gap: s(8), paddingHorizontal: s(2) }}
    >
      <Text
        accessibilityRole="header"
        numberOfLines={1}
        style={{
          color,
          fontSize: s(12.5),
          lineHeight: s(17),
          fontWeight: "600",
          flexShrink: 1,
        }}
      >
        {label}
      </Text>
      {meta ? (
        <Text style={{ color: colors.foregroundMuted, fontSize: s(12), lineHeight: s(16) }}>
          {meta}
        </Text>
      ) : null}
    </View>
  );
  return (
    <View style={{ flex: 1, minWidth: 0, gap: s(14) }}>
      <View
        style={{
          flexDirection: "row",
          alignItems: "center",
          gap: s(8),
          paddingBottom: s(10),
          borderBottomWidth: 1,
          borderBottomColor: colors.border,
        }}
      >
        <Text
          accessibilityRole="header"
          style={{
            color: colors.foreground,
            fontSize: s(15),
            lineHeight: s(20),
            fontWeight: "600",
          }}
        >
          {title}
        </Text>
        <Text
          accessibilityLabel={`${count} conversations`}
          style={{
            color: colors.foregroundMuted,
            fontSize: s(13),
            lineHeight: s(18),
            fontWeight: "500",
          }}
        >
          {count}
        </Text>
      </View>
      {runs.length ? (
        <>
          {sections.projects.map((project) => (
            <View
              key={project.key}
              style={{
                gap: s(8),
                padding: s(8),
                borderRadius: s(CARD_RADIUS + 4),
                borderWidth: 1,
                borderColor: tint(project),
                backgroundColor: colors.surface1,
              }}
            >
              <View
                pointerEvents="none"
                accessibilityElementsHidden
                importantForAccessibility="no-hide-descendants"
                style={{
                  position: "absolute",
                  inset: 0,
                  borderRadius: s(CARD_RADIUS + 4),
                  backgroundColor: tint(project),
                  opacity: 0.14,
                }}
              />
              <View
                style={{
                  flexDirection: "row",
                  alignItems: "center",
                  gap: s(8),
                  paddingHorizontal: s(4),
                  paddingTop: s(2),
                }}
              >
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel={`New conversation in ${project.name}`}
                  disabled={!onOpenProject}
                  onPress={() => onOpenProject?.(project.runs[0])}
                  style={({ pressed }) => ({
                    flex: 1,
                    flexDirection: "row",
                    alignItems: "center",
                    gap: s(8),
                    borderRadius: s(CONTROL_RADIUS),
                    opacity: pressed ? 0.6 : 1,
                  })}
                >
                  <ProjectMark
                    name={project.name}
                    color={mark(project)}
                    size={s(20)}
                    theme={theme}
                  />
                  <View style={{ flex: 1 }}>
                    {heading(
                      project.name,
                      groupCount(project) > 1 ? String(groupCount(project)) : null,
                      colors.foreground,
                    )}
                  </View>
                </Pressable>
              </View>
              {project.runs.map(renderCard)}
            </View>
          ))}
        </>
      ) : (
        <View
          style={{
            minHeight: s(88),
            alignItems: "center",
            justifyContent: "center",
            padding: s(16),
            borderRadius: s(CARD_RADIUS),
            borderWidth: 1,
            borderStyle: "dashed",
            borderColor: colors.border,
          }}
        >
          <Text
            style={{
              color: colors.foregroundMuted,
              fontSize: s(13),
              lineHeight: s(18),
              textAlign: "center",
            }}
          >
            {emptyMessage}
          </Text>
        </View>
      )}
    </View>
  );
});

const NO_RUNS: BoardRun[] = [];
const NO_HUES: Record<string, number> = {};

export function BoardPage({ host, theme, layout, navigation }: PluginSurfaceProps) {
  const palette = useSettings(projectColors);
  const savingPalette = useRef(false);
  // Host settings keep the size across plugin reloads and restarts without changing host appearance.
  const sizeSettings = useSettings(boardSize);
  const orbConfig = useSettings(orbSettings);
  if (orbConfig.status === "ready") lastSettings.orb = orbConfig.values;
  const orb = orbConfig.status === "ready" ? orbConfig.values : (lastSettings.orb ?? null);
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
  const readBoard = useRpc(boardRpc);
  const removeRun = useRpc(removeRunRpc);
  const setStarred = useRpc(starRunRpc);
  const board = useQuery({
    queryKey: ["board", host.id],
    queryFn: () => readBoard({}),
    retry: false,
    refetchInterval: 2_000,
    refetchOnWindowFocus: false,
    // Opening a thread unmounts Board. Drop its cached snapshot so a thread's Remove
    // action cannot briefly show the removed card when Board mounts again.
    gcTime: 0,
  });
  const onStar = useStableCallback(async (id: string, starred: boolean) => {
    const result = await setStarred({ id, starred, observingSince: board.data!.observingSince });
    if (!result.updated) throw new Error("Run changed. Refresh and retry.");
    await board.refetch({ throwOnError: true });
  });
  const runs = board.data?.runs ?? NO_RUNS;
  useEffect(() => {
    if (palette.status !== "ready" || palette.saving || palette.saveError || savingPalette.current)
      return;
    const next = allocateColors(
      palette.values.hues,
      runs.flatMap((run) => (run.projectId ? [run.projectId] : [])),
    );
    if (next === palette.values.hues) return;
    savingPalette.current = true;
    void palette.save({ hues: next }, palette.revision).finally(() => {
      savingPalette.current = false;
    });
  }, [palette, board.dataUpdatedAt]);
  if (palette.status === "ready") lastSettings.hues = palette.values.hues;
  const projectPalette =
    palette.status === "ready" ? palette.values.hues : (lastSettings.hues ?? NO_HUES);

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
    if (run.cwd === undefined || !workspaceActions.open) {
      setProjectError("This project has no folder to start a conversation in.");
      return;
    }
    workspaceActions.open({
      serverId: host.id,
      cwd: run.cwd,
      name: run.project,
      projectId: run.projectId,
    });
  });
  const openAgentStable = useStableCallback((agentId: string) => {
    navigation?.openAgent({ agentId, anchor: "latest-prompt" });
  });
  const openAgent = navigation ? openAgentStable : undefined;
  const onRemove = useStableCallback(async (id: string) => {
    const run = runs.find((item) => item.id === id);
    if (!run) return;
    const removed = await removeFinishedRun(
      run,
      board.data!.observingSince,
      (scope) => removeRun({ id, observingSince: scope, endedAt: run.endedAt }),
      () => readBoard({}),
    );
    if (!removed) throw new Error("Run changed. Refresh and retry.");
    await board.refetch({ throwOnError: true });
  });
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const connection = useClock((now) =>
    boardConnectionState({
      hasData: Boolean(board.data),
      isError: board.isError,
      isPaused: board.isPaused,
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
  const pagePad = layout.compact ? 16 : 32;

  return (
    <View style={{ flex: 1, backgroundColor: colors.surface0 }}>
      {/* Outside scaled scroll content so repeated clicks keep the same target. */}
      <View
        style={{
          flexDirection: "row",
          flexWrap: "wrap",
          justifyContent: "flex-end",
          alignItems: "center",
          gap: 12,
          paddingHorizontal: pagePad,
          paddingTop: 12,
          paddingBottom: 4,
        }}
      >
        <BoardViewSwitch view={view} onChange={setView} theme={theme} />
        <BoardSizeControl size={size} onChange={changeSize} theme={theme} />
      </View>
      <ScrollView
        style={{ flex: 1, backgroundColor: colors.surface0 }}
        contentContainerStyle={{
          flexGrow: 1,
          paddingHorizontal: s(pagePad),
          paddingTop: s(8),
          paddingBottom: s(40),
          gap: s(layout.compact ? 20 : 28),
          maxWidth: 1440,
          width: "100%",
          alignSelf: "center",
        }}
      >
        <View
          style={{
            flexDirection: "row",
            alignItems: "flex-end",
            justifyContent: "space-between",
            gap: s(12),
          }}
        >
          <View style={{ flexShrink: 1, gap: s(2) }}>
            <Text
              accessibilityRole="header"
              style={{
                color: colors.foreground,
                fontSize: s(layout.compact ? 22 : 24),
                lineHeight: s(layout.compact ? 28 : 30),
                fontWeight: "700",
                letterSpacing: -0.4,
              }}
            >
              Board
            </Text>
            <Text style={{ color: colors.foregroundMuted, fontSize: s(13), lineHeight: s(18) }}>
              {view === "recaps"
                ? "Recaps from the last 7 days, by project"
                : "Running and recently finished conversations"}
            </Text>
          </View>
          <View
            accessibilityLabel={`Connection ${connectionLabel}`}
            style={{ flexDirection: "row", alignItems: "center", gap: s(6), paddingBottom: s(2) }}
          >
            <View
              accessibilityElementsHidden
              style={{
                width: s(6),
                height: s(6),
                borderRadius: s(3),
                backgroundColor: connectionColor,
              }}
            />
            <Text
              style={{
                color: connection === "error" ? colors.statusDanger : colors.foregroundMuted,
                fontSize: s(12.5),
                lineHeight: s(16),
                fontWeight: "500",
              }}
            >
              {connectionLabel}
            </Text>
          </View>
        </View>

        {palette.status === "error" || palette.status === "invalid" || palette.saveError ? (
          <Pressable accessibilityRole="button" onPress={() => void palette.reload()}>
            <Text style={{ color: colors.statusWarning, fontSize: s(13), lineHeight: s(18) }}>
              Could not save project colors. Tap to retry.
            </Text>
          </Pressable>
        ) : null}
        {view === "recaps" ? (
          <RecapsView
            hostId={host.id}
            theme={theme}
            scale={scale}
            hues={projectPalette}
            onOpen={openAgent}
          />
        ) : initialLoading ? (
          <View
            style={{
              flex: 1,
              minHeight: s(220),
              alignItems: "center",
              justifyContent: "center",
              gap: s(12),
            }}
          >
            <ActivityIndicator color={colors.accent} />
            <Text style={{ color: colors.foregroundMuted, fontSize: s(13), lineHeight: s(18) }}>
              Loading runs…
            </Text>
          </View>
        ) : unavailable ? (
          <View
            accessibilityRole="alert"
            style={{
              minHeight: s(220),
              alignItems: "center",
              justifyContent: "center",
              gap: s(12),
              padding: s(24),
              borderRadius: s(CARD_RADIUS),
              borderWidth: 1,
              borderColor: colors.border,
              backgroundColor: colors.surface1,
            }}
          >
            <Text
              style={{
                color: colors.foreground,
                fontSize: s(16),
                lineHeight: s(22),
                fontWeight: "600",
              }}
            >
              {connection === "offline" ? "Board is offline" : "Board is unavailable"}
            </Text>
            <Text
              style={{
                color: colors.foregroundMuted,
                fontSize: s(13),
                textAlign: "center",
                lineHeight: s(19),
              }}
            >
              {connection === "offline"
                ? "Reconnect to the host, then retry."
                : "Check the host connection, then retry."}
            </Text>
            <Pressable
              accessibilityRole="button"
              accessibilityLabel="Retry loading Board"
              disabled={board.isFetching}
              onPress={() => void board.refetch()}
              style={{
                paddingHorizontal: s(16),
                paddingVertical: s(9),
                borderRadius: s(CONTROL_RADIUS),
                backgroundColor: colors.accent,
              }}
            >
              <Text
                style={{
                  color: colors.accentForeground,
                  fontSize: s(13),
                  lineHeight: s(18),
                  fontWeight: "600",
                }}
              >
                {board.isFetching ? "Retrying…" : "Retry"}
              </Text>
            </Pressable>
          </View>
        ) : (
          <>
            {projectError ? (
              <Text
                accessibilityRole="alert"
                style={{ color: colors.statusDanger, fontSize: s(13), lineHeight: s(18) }}
              >
                {projectError}
              </Text>
            ) : null}
            {snapshotWarning ? (
              <View
                accessibilityRole="alert"
                style={{
                  flexDirection: layout.compact ? "column" : "row",
                  alignItems: layout.compact ? "stretch" : "center",
                  justifyContent: "space-between",
                  gap: s(10),
                  paddingVertical: s(10),
                  paddingHorizontal: s(14),
                  borderRadius: s(CARD_RADIUS),
                  borderWidth: 1,
                  borderColor: connectionColor,
                  backgroundColor: colors.surface1,
                }}
              >
                <Text
                  style={{ flex: 1, color: connectionColor, fontSize: s(13), lineHeight: s(18) }}
                >
                  {snapshotWarning}
                </Text>
                <Pressable
                  accessibilityRole="button"
                  disabled={board.isFetching}
                  onPress={() => void board.refetch()}
                  style={{
                    paddingHorizontal: s(12),
                    paddingVertical: s(6),
                    borderRadius: s(CONTROL_RADIUS),
                    backgroundColor: colors.surface2,
                  }}
                >
                  <Text
                    style={{
                      color: colors.foreground,
                      fontSize: s(13),
                      lineHeight: s(18),
                      fontWeight: "600",
                    }}
                  >
                    {board.isFetching ? "Retrying…" : "Retry"}
                  </Text>
                </Pressable>
              </View>
            ) : null}
            <View
              style={{
                flexGrow: layout.compact ? 0 : 1,
                flexDirection: layout.compact ? "column" : "row",
                alignItems: "stretch",
                gap: s(layout.compact ? 28 : 32),
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
                onRemove={onRemove}
                onStar={onStar}
                emptyMessage="No conversations are running."
                theme={theme}
                onOpen={openAgent}
                onOpenProject={onOpenProject}
              />
              {layout.compact ? null : (
                <View
                  accessibilityElementsHidden
                  importantForAccessibility="no-hide-descendants"
                  style={{ width: 1, backgroundColor: colors.border }}
                />
              )}
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
                onOpenProject={onOpenProject}
                onRemove={onRemove}
              />
            </View>
            <Text style={{ color: colors.foregroundMuted, fontSize: s(12), lineHeight: s(16) }}>
              Last 50 finished conversations
            </Text>
          </>
        )}
      </ScrollView>
    </View>
  );
}
