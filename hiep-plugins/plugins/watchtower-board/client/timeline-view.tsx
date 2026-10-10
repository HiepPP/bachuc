import type { PluginTheme } from "@getpaseo/plugin";
import { Fragment, useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";
import type { Timeline, TimelineBar } from "./timeline";
import {
  BAR_HEIGHT,
  type BarRect,
  clockAt,
  nowLabel,
  parallelCaption,
  type SegmentTone,
  shortId,
  timelineGeometry,
} from "./timeline-geometry";

interface BranchTimelineProps {
  model: Timeline;
  nowMinute: number | null;
  started: string | null;
  theme: PluginTheme;
  onSelectTask?: (id: string) => void;
  // The run has stopped, so its now mark is the last activity.
  stopped?: boolean;
}

function describe(bar: TimelineBar, started: string | null): string {
  const minutes = Math.round(bar.end - bar.start);
  if (bar.kind === "done") return `${bar.id}, done in ${minutes} min`;
  if (bar.kind === "running") return `${bar.id}, running ${minutes} min`;
  const at = clockAt(started, bar.start);
  const when = at ? ` at ${at}` : "";
  return bar.kind === "blocked"
    ? `${bar.id}, blocked, planned${when}`
    : `${bar.id}, planned${when}`;
}

export function BranchTimeline({
  model,
  nowMinute,
  started,
  theme,
  onSelectTask,
}: BranchTimelineProps) {
  const [width, setWidth] = useState(0);
  const geometry = useMemo(
    () =>
      width > 0 && model.bars.length > 0
        ? timelineGeometry(model, width, nowMinute, started)
        : null,
    [model, width, nowMinute, started],
  );
  const { colors } = theme;
  const tone: Record<SegmentTone, string> = {
    done: colors.statusSuccess,
    active: colors.accent,
    pending: colors.foregroundMuted,
  };
  const barStyle = (bar: BarRect) => {
    if (bar.kind === "done") return { backgroundColor: colors.statusSuccess, opacity: 0.8 };
    // An accent outline with a light tint, so a running bar stays distinct from a done bar even in
    // a theme whose accent is green like statusSuccess.
    if (bar.kind === "running")
      return { borderWidth: 1.5, borderColor: colors.accent, overflow: "hidden" as const };
    return {
      borderWidth: 1,
      borderStyle: "dashed" as const,
      borderColor: bar.kind === "blocked" ? colors.statusDanger : colors.foregroundMuted,
    };
  };
  const byId = new Map(model.bars.map((bar) => [bar.id, bar]));
  const { stats } = model;
  const statItems = [
    { value: `${stats.averageMinutes} min`, caption: "average task", warn: false },
    {
      value: stats.longest ? `${stats.longest.minutes} min` : "-",
      caption: stats.longest ? `longest, ${shortId(stats.longest.id)}` : "longest task",
      warn: false,
    },
    {
      value: stats.overAverage ? shortId(stats.overAverage.id) : "None",
      caption: stats.overAverage ? `${stats.overAverage.minutes} min over average` : "over average",
      warn: Boolean(stats.overAverage),
    },
    { value: String(stats.maxParallel), caption: parallelCaption(stats.maxParallel), warn: false },
  ];

  return (
    <View>
      <View onLayout={(event) => setWidth(event.nativeEvent.layout.width)}>
        {model.bars.length === 0 ? (
          <Text style={{ color: colors.foregroundMuted, fontSize: 12.5 }}>No timeline yet.</Text>
        ) : null}
        {geometry ? (
          <View style={{ height: geometry.height }}>
            {geometry.breaks.map((gap) => (
              <Fragment key={`break-${gap.x}`}>
                <View
                  accessible
                  accessibilityLabel={`Idle ${gap.label}, not drawn to scale`}
                  style={{
                    position: "absolute",
                    left: gap.x,
                    top: 0,
                    width: gap.width,
                    height: geometry.axisY,
                    borderLeftWidth: 1,
                    borderRightWidth: 1,
                    borderStyle: "dashed",
                    borderColor: colors.border,
                    backgroundColor: colors.surface2,
                  }}
                />
                <Text
                  numberOfLines={1}
                  style={{
                    position: "absolute",
                    left: gap.x,
                    width: gap.width,
                    // The label row, where no dependency link runs through it.
                    top: 0,
                    textAlign: "center",
                    fontSize: 10,
                    color: colors.foregroundMuted,
                  }}
                >
                  {gap.label}
                </Text>
              </Fragment>
            ))}
            {geometry.segments.map((segment, index) => (
              <View
                key={`segment-${index}`}
                style={{
                  position: "absolute",
                  left: segment.x,
                  top: segment.y,
                  width: segment.width,
                  height: segment.height,
                  backgroundColor: tone[segment.tone],
                  opacity: segment.tone === "pending" ? 0.45 : 1,
                }}
              />
            ))}
            {geometry.bars.map((bar) => {
              const source = byId.get(bar.id);
              return (
                <Fragment key={bar.id}>
                  <Pressable
                    accessibilityRole="button"
                    accessibilityLabel={source ? describe(source, started) : bar.id}
                    disabled={!onSelectTask}
                    hitSlop={{ top: 8, bottom: 8 }}
                    onPress={() => onSelectTask?.(bar.id)}
                    style={{
                      position: "absolute",
                      left: bar.x,
                      top: bar.y,
                      width: bar.width,
                      height: BAR_HEIGHT,
                      borderRadius: 3,
                      ...barStyle(bar),
                    }}
                  >
                    {bar.kind === "running" ? (
                      <View
                        pointerEvents="none"
                        style={{
                          position: "absolute",
                          top: 0,
                          right: 0,
                          bottom: 0,
                          left: 0,
                          backgroundColor: colors.accent,
                          opacity: 0.22,
                        }}
                      />
                    ) : null}
                  </Pressable>
                  {bar.expected ? (
                    <View
                      style={{
                        position: "absolute",
                        left: bar.expected.x,
                        top: bar.expected.y,
                        width: bar.expected.width,
                        height: BAR_HEIGHT,
                        borderRadius: 3,
                        borderWidth: 1,
                        borderStyle: "dashed",
                        borderColor: colors.accent,
                      }}
                    />
                  ) : null}
                </Fragment>
              );
            })}
            {geometry.labels.map((label) => (
              <Text
                key={`label-${label.id}`}
                numberOfLines={1}
                style={{
                  position: "absolute",
                  left: label.x,
                  top: label.y,
                  fontSize: 10,
                  fontVariant: ["tabular-nums"],
                  fontWeight: label.kind === "running" ? "600" : "400",
                  color:
                    label.kind === "running"
                      ? colors.foreground
                      : label.kind === "blocked"
                        ? colors.statusDanger
                        : colors.foregroundMuted,
                }}
              >
                {label.text}
              </Text>
            ))}
            {geometry.nowX !== null ? (
              <View
                style={{
                  position: "absolute",
                  left: geometry.nowX,
                  top: 0,
                  width: 1,
                  height: geometry.axisY,
                  backgroundColor: colors.foregroundMuted,
                  opacity: 0.6,
                }}
              />
            ) : null}
            <View
              style={{
                position: "absolute",
                left: 0,
                right: 0,
                top: geometry.axisY,
                height: 1,
                backgroundColor: colors.border,
              }}
            />
            {geometry.ticks.map((tick) => (
              <Text
                key={`tick-${tick.x}`}
                style={{
                  position: "absolute",
                  left: tick.x - 18,
                  width: 36,
                  top: geometry.axisY + 3,
                  textAlign: "center",
                  fontSize: 10,
                  fontVariant: ["tabular-nums"],
                  color: colors.foregroundMuted,
                }}
              >
                {tick.label}
              </Text>
            ))}
            {geometry.days.map((day) => (
              <Text
                key={`day-${day.x}`}
                numberOfLines={1}
                style={{
                  position: "absolute",
                  left: day.x,
                  top: geometry.axisY + 3,
                  fontSize: 10,
                  fontWeight: "600",
                  color: colors.foreground,
                }}
              >
                {day.label}
              </Text>
            ))}
          </View>
        ) : null}
      </View>
      <View
        style={{
          flexDirection: "row",
          gap: 12,
          marginTop: 14,
          paddingTop: 12,
          borderTopWidth: 1,
          borderTopColor: colors.border,
        }}
      >
        {statItems.map((item) => (
          <View key={item.caption} style={{ flex: 1, minWidth: 0 }}>
            <Text
              numberOfLines={1}
              style={{
                fontSize: 17,
                fontWeight: "600",
                fontVariant: ["tabular-nums"],
                color: item.warn ? colors.statusWarning : colors.foreground,
              }}
            >
              {item.value}
            </Text>
            <Text numberOfLines={1} style={{ fontSize: 11.5, color: colors.foregroundMuted }}>
              {item.caption}
            </Text>
          </View>
        ))}
      </View>
    </View>
  );
}

const STRIP_HEIGHT = 14;

// The narrow form of the timeline: every bar on one row, with the same x and breaks as the Gantt.
export function TimelineStrip({
  model,
  nowMinute,
  started,
  theme,
  stopped = false,
}: BranchTimelineProps) {
  const [width, setWidth] = useState(0);
  const geometry = useMemo(
    () =>
      width > 0 && model.bars.length > 0
        ? timelineGeometry(model, width, nowMinute, started)
        : null,
    [model, width, nowMinute, started],
  );
  const { colors } = theme;
  const fill = (kind: BarRect["kind"]) =>
    kind === "done"
      ? { backgroundColor: colors.statusSuccess, opacity: 0.55 }
      : kind === "running"
        ? { backgroundColor: colors.accent }
        : {
            borderWidth: 1,
            borderStyle: "dashed" as const,
            borderColor: kind === "blocked" ? colors.statusDanger : colors.foregroundMuted,
          };
  const first = clockAt(started, model.axis.start);
  const now = nowMinute === null ? null : clockAt(started, nowMinute);
  const finish = model.finishMinute === null ? null : clockAt(started, model.finishMinute);
  const muted = {
    fontSize: 11,
    color: colors.foregroundMuted,
    fontVariant: ["tabular-nums" as const],
  };
  return (
    <View style={{ gap: 6 }}>
      <View
        onLayout={(event) => setWidth(event.nativeEvent.layout.width)}
        style={{ height: STRIP_HEIGHT, borderRadius: 4, backgroundColor: colors.surface2 }}
      >
        {geometry?.bars.map((bar) => (
          <View
            key={bar.id}
            style={{
              position: "absolute",
              left: bar.x,
              top: 0,
              width: bar.width,
              height: STRIP_HEIGHT,
              ...fill(bar.kind),
            }}
          />
        ))}
        {geometry?.breaks.map((gap) => (
          <View
            key={`break-${gap.x}`}
            accessible
            accessibilityLabel={`Idle ${gap.label}`}
            style={{
              position: "absolute",
              left: gap.x,
              top: 0,
              width: gap.width,
              height: STRIP_HEIGHT,
              borderLeftWidth: 1,
              borderRightWidth: 1,
              borderColor: colors.border,
              backgroundColor: colors.surface0,
            }}
          />
        ))}
        {geometry?.nowX != null ? (
          <View
            style={{
              position: "absolute",
              left: geometry.nowX,
              top: -2,
              width: 2,
              height: STRIP_HEIGHT + 4,
              backgroundColor: colors.statusDanger,
            }}
          />
        ) : null}
      </View>
      <View style={{ flexDirection: "row", justifyContent: "space-between", gap: 8 }}>
        <Text style={muted}>{first ?? ""}</Text>
        <Text style={muted}>{nowLabel(now, stopped)}</Text>
        <Text style={muted}>{finish ? `finish ${finish}` : ""}</Text>
      </View>
    </View>
  );
}
