import { Icon } from "@getpaseo/plugin/client/react-native";
import { useEffect, useRef, useState } from "react";
import { AccessibilityInfo, Animated, Easing, Platform, Pressable, Text, View } from "react-native";
import type { OverviewRunEntry } from "../shared/overview";
import type { BuilderMatch, RunAgent } from "./builders";
import { MONO, type ThemeProps } from "./overview-cells";
import type { PlanTree as PlanTreeModel, PlanTreeRow } from "./plan-tree";

// The plan tree of mock D v2: a lead row, then waves of TASK rows hanging off one spine.

const ICON_BOX = 18;
const SPINE_WIDTH = 2;
// Left edge of the spine: the center of the state icon column of the wave header.
const SPINE_INSET = ICON_BOX / 2 - SPINE_WIDTH / 2;
const SPINE_GAP = 14;
const COMMIT = /\b(?=[0-9a-f]*\d)[0-9a-f]{7,40}\b/;

/** The short commit hash each TASK's log row names. Rows with no hash are left out. */
export function commitShas(log: readonly OverviewRunEntry[]): Record<string, string> {
  const shas: Record<string, string> = {};
  for (const entry of log) {
    const sha = COMMIT.exec(`${entry.detail} ${entry.result}`)?.[0];
    if (sha) shas[entry.task] = sha.slice(0, 7);
  }
  return shas;
}

function ago(ms: number | null): string | null {
  if (ms === null) return null;
  const seconds = Math.round(ms / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  return minutes < 60 ? `${minutes}m ago` : `${Math.round(minutes / 60)}h ago`;
}

function useReducedMotion(): boolean {
  // Static until the system answers, so a reduced-motion owner never sees the spinner turn.
  const [reduced, setReduced] = useState(true);
  useEffect(() => {
    let alive = true;
    void AccessibilityInfo.isReduceMotionEnabled().then((value) => alive && setReduced(value));
    const subscription = AccessibilityInfo.addEventListener("reduceMotionChanged", setReduced);
    return () => {
      alive = false;
      subscription.remove();
    };
  }, []);
  return reduced;
}

// The only animation of the tree: one turning icon per running row, off under reduced motion.
function Spinner({ color }: { color: string }) {
  const reduced = useReducedMotion();
  const turn = useRef(new Animated.Value(0)).current;
  useEffect(() => {
    if (reduced) return;
    turn.setValue(0);
    const loop = Animated.loop(
      Animated.timing(turn, {
        toValue: 1,
        duration: 1000,
        easing: Easing.linear,
        useNativeDriver: Platform.OS !== "web",
      }),
    );
    loop.start();
    return () => loop.stop();
  }, [reduced, turn]);
  const rotate = turn.interpolate({ inputRange: [0, 1], outputRange: ["0deg", "360deg"] });
  return (
    <Animated.View style={{ transform: [{ rotate }] }}>
      <Icon name="LoaderCircle" size={16} color={color} />
    </Animated.View>
  );
}

type RowState = "done" | "running" | "waiting" | "planned" | "blocked";

function StateIcon({ state, theme }: { state: RowState } & ThemeProps) {
  const { colors } = theme;
  let glyph;
  if (state === "done")
    glyph = <Icon name="CircleCheck" size={16} color={colors.foregroundMuted} />;
  else if (state === "running") glyph = <Spinner color={colors.accent} />;
  else if (state === "waiting") glyph = <Icon name="Hand" size={16} color={colors.statusWarning} />;
  else if (state === "blocked")
    glyph = <Icon name="CircleAlert" size={16} color={colors.foreground} />;
  else
    glyph = (
      <View
        style={{
          width: 14,
          height: 14,
          borderRadius: 7,
          borderWidth: 1.5,
          borderStyle: "dashed",
          borderColor: colors.foregroundMuted,
        }}
      />
    );
  return (
    <View
      style={{ width: ICON_BOX, height: ICON_BOX, alignItems: "center", justifyContent: "center" }}
    >
      {glyph}
    </View>
  );
}

function OpenThread({
  agentId,
  onOpenAgent,
  theme,
}: { agentId: string; onOpenAgent(agentId: string): void } & ThemeProps) {
  const { colors } = theme;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel="Open thread"
      onPress={() => onOpenAgent(agentId)}
      style={({ pressed }) => ({
        paddingHorizontal: 10,
        paddingVertical: 5,
        borderRadius: 7,
        borderWidth: 1,
        borderColor: colors.border,
        backgroundColor: colors.surface0,
        opacity: pressed ? 0.6 : 1,
      })}
    >
      <Text style={{ fontSize: 12, fontWeight: "500", color: colors.foreground }}>Open thread</Text>
    </Pressable>
  );
}

function secondaryLine(
  row: PlanTreeRow,
  state: RowState,
  builder: RunAgent | undefined,
  taskClass: string | null | undefined,
  sha: string | undefined,
): string {
  const group = row.group ? `Group ${row.group}` : null;
  const letter = row.group ? `Builder ${row.group}` : "Builder";
  const plain = [group, taskClass].filter(Boolean).join(", ");
  if (state === "done") return sha ? `committed ${sha}` : plain;
  if (state === "waiting") return `${letter} asks permission`;
  if (state === "running" && builder) {
    const last = ago(builder.activeAgo);
    const word = builder.state === "failed" ? "failed" : builder.state === "idle" ? "idle" : null;
    return [letter, word, last ? `active ${last}` : null].filter(Boolean).join(", ");
  }
  return state === "blocked" ? ["Blocked", plain].filter(Boolean).join(", ") : plain;
}

function TaskRow({
  row,
  builder,
  taskClass,
  sha,
  narrow,
  theme,
  onSelectTask,
  onOpenAgent,
}: {
  row: PlanTreeRow;
  builder: RunAgent | undefined;
  taskClass: string | null | undefined;
  sha: string | undefined;
  narrow: boolean;
  onSelectTask?(id: string): void;
  onOpenAgent?(agentId: string): void;
} & ThemeProps) {
  const { colors } = theme;
  const state: RowState =
    row.kind === "running" && builder?.state === "waiting" ? "waiting" : row.kind;
  const waiting = state === "waiting";
  const secondary = secondaryLine(row, state, builder, taskClass, sha);
  return (
    <View>
      {narrow ? null : (
        <View
          pointerEvents="none"
          style={{
            position: "absolute",
            left: -SPINE_GAP - SPINE_WIDTH,
            top: "50%",
            marginTop: -SPINE_WIDTH / 2,
            width: SPINE_GAP + SPINE_WIDTH,
            height: SPINE_WIDTH,
            backgroundColor: colors.border,
          }}
        />
      )}
      <View
        style={{
          flexDirection: "row",
          alignItems: "center",
          gap: 8,
          paddingVertical: 7,
          paddingHorizontal: 8,
          borderRadius: 8,
          overflow: "hidden",
          borderWidth: 1,
          borderColor: waiting ? colors.statusWarning : "transparent",
        }}
      >
        {waiting ? (
          <View
            pointerEvents="none"
            style={{
              position: "absolute",
              top: 0,
              right: 0,
              bottom: 0,
              left: 0,
              backgroundColor: colors.statusWarning,
              opacity: 0.08,
            }}
          />
        ) : null}
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`${row.id}, ${row.title}, ${state}`}
          disabled={!onSelectTask}
          onPress={() => onSelectTask?.(row.id)}
          style={({ pressed }) => ({
            flex: 1,
            minWidth: 0,
            flexDirection: "row",
            alignItems: "center",
            gap: 8,
            opacity: pressed ? 0.6 : 1,
          })}
        >
          <StateIcon state={state} theme={theme} />
          <View style={{ flex: 1, minWidth: 0 }}>
            <View style={{ flexDirection: "row", alignItems: "baseline", gap: 8 }}>
              <Text style={{ fontFamily: MONO, fontSize: 11, color: colors.foregroundMuted }}>
                {row.id}
              </Text>
              <Text
                numberOfLines={1}
                style={{
                  flex: 1,
                  fontSize: 13,
                  fontWeight: "500",
                  color: state === "planned" ? colors.foregroundMuted : colors.foreground,
                }}
              >
                {row.title}
              </Text>
            </View>
            {secondary ? (
              <Text
                numberOfLines={1}
                style={{
                  fontSize: 11.5,
                  marginTop: 1,
                  color: waiting ? colors.statusWarning : colors.foregroundMuted,
                }}
              >
                {secondary}
              </Text>
            ) : null}
          </View>
          {row.time ? (
            <Text
              style={{
                fontFamily: MONO,
                fontSize: 11,
                fontWeight: row.over ? "700" : "400",
                color: state === "running" ? colors.foreground : colors.foregroundMuted,
              }}
            >
              {row.time}
            </Text>
          ) : null}
        </Pressable>
        {waiting && builder && onOpenAgent ? (
          <OpenThread agentId={builder.id} onOpenAgent={onOpenAgent} theme={theme} />
        ) : null}
      </View>
    </View>
  );
}

function LeadRow({
  lead,
  builderCount,
  planTitle,
  runState,
  onOpenAgent,
  theme,
}: {
  lead: RunAgent | null;
  builderCount: number;
  planTitle?: string | null;
  runState?: string | null;
  onOpenAgent?(agentId: string): void;
} & ThemeProps) {
  const { colors } = theme;
  // Q-036 default: with no lead agent, show the plan title and the run state, with no button.
  const title = lead ? "Lead agent, implement" : (planTitle ?? "Lead agent, implement");
  let line: string;
  if (lead) {
    const state =
      lead.state === "waiting"
        ? "asks permission"
        : lead.state === "running"
          ? "running"
          : lead.state === "failed"
            ? "failed"
            : "idle";
    const last = ago(lead.activeAgo);
    line = [
      lead.title,
      state,
      last ? `active ${last}` : null,
      builderCount > 0
        ? `waiting on ${builderCount} ${builderCount === 1 ? "builder" : "builders"}`
        : "no builder running",
    ]
      .filter(Boolean)
      .join(", ");
  } else {
    line = runState ?? "No lead agent found";
  }
  return (
    <View
      style={{
        flexDirection: "row",
        alignItems: "center",
        gap: 10,
        paddingVertical: 8,
        paddingHorizontal: 10,
        marginBottom: 10,
        borderRadius: 10,
        borderWidth: 1,
        borderColor: colors.border,
        backgroundColor: colors.surface1,
      }}
    >
      <View
        style={{
          width: 24,
          height: 24,
          borderRadius: 6,
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: colors.surface2,
        }}
      >
        <Icon name="Bot" size={14} color={colors.foreground} />
      </View>
      <View style={{ flex: 1, minWidth: 0 }}>
        <Text
          numberOfLines={1}
          style={{ fontSize: 13, fontWeight: "600", color: colors.foreground }}
        >
          {title}
        </Text>
        <Text
          numberOfLines={1}
          style={{ fontSize: 11.5, marginTop: 1, color: colors.foregroundMuted }}
        >
          {line}
        </Text>
      </View>
      {lead && onOpenAgent ? (
        <OpenThread agentId={lead.id} onOpenAgent={onOpenAgent} theme={theme} />
      ) : null}
    </View>
  );
}

export interface PlanTreeProps extends ThemeProps {
  tree: PlanTreeModel;
  builders: BuilderMatch["byTask"];
  lead: BuilderMatch["lead"];
  // Drops the spine and stacks the waves as plain lists.
  narrow: boolean;
  onSelectTask?(id: string): void;
  onOpenAgent?(agentId: string): void;
  // Shown on the lead row when no lead agent is found (Q-036).
  planTitle?: string | null;
  runState?: string | null;
  // The Tracker class per TASK, for the "Group A, code" line.
  classes?: Readonly<Record<string, string | null>>;
  // The commit hash per TASK, from `commitShas`.
  commits?: Readonly<Record<string, string>>;
}

export function PlanTree({
  tree,
  builders,
  lead,
  narrow,
  theme,
  onSelectTask,
  onOpenAgent,
  planTitle,
  runState,
  classes,
  commits,
}: PlanTreeProps) {
  const { colors } = theme;
  // Every wave stays open, done waves too: the owner reads the whole plan without a click.
  const active = new Set(
    Object.values(builders)
      .filter((builder) => builder.state === "running" || builder.state === "waiting")
      .map((builder) => builder.id),
  );
  return (
    <View accessibilityRole="list">
      <LeadRow
        lead={lead}
        builderCount={active.size}
        planTitle={planTitle}
        runState={runState}
        onOpenAgent={onOpenAgent}
        theme={theme}
      />
      {tree.waves.map((wave) => {
        return (
          <View key={wave.number} style={{ marginBottom: 10 }}>
            <View
              style={{
                flexDirection: "row",
                alignItems: "baseline",
                gap: 8,
                paddingVertical: 4,
                paddingHorizontal: 8,
              }}
            >
              <Text style={{ fontSize: 12.5, fontWeight: "600", color: colors.foreground }}>
                Wave {wave.number}
              </Text>
              <Text
                numberOfLines={1}
                style={{ flex: 1, fontSize: 11.5, color: colors.foregroundMuted }}
              >
                {wave.header}
              </Text>
            </View>
            <View
              style={
                narrow
                  ? undefined
                  : {
                      marginLeft: SPINE_INSET,
                      paddingLeft: SPINE_GAP,
                      borderLeftWidth: SPINE_WIDTH,
                      borderLeftColor: colors.border,
                    }
              }
            >
              {wave.rows.map((row) => (
                <TaskRow
                  key={row.id}
                  row={row}
                  builder={builders[row.id]}
                  taskClass={classes?.[row.id]}
                  sha={commits?.[row.id]}
                  narrow={narrow}
                  theme={theme}
                  onSelectTask={onSelectTask}
                  onOpenAgent={onOpenAgent}
                />
              ))}
            </View>
          </View>
        );
      })}
    </View>
  );
}
