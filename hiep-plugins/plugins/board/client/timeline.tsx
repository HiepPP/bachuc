import type { PluginHostSummary, PluginSurfaceProps } from "@getpaseo/plugin/client";
import { useQueries } from "@tanstack/react-query";
import { useMemo, useState, type ReactNode } from "react";
import { ActivityIndicator, Pressable, Text, View, type TextStyle } from "react-native";
import { projectColors } from "../shared/project-colors";
import {
  clockText,
  durationText,
  timelineModel,
  timelineNotes,
  timelineRpc,
  type TimelineBar,
  type TimelineKind,
  type TimelineModel,
  type TimelineRow,
} from "../shared/timeline";
import { useClock } from "./clock";
import { useHostSettings } from "./host-settings";
import type { BoardHostClient } from "./hosts";
import { repoLabel, repoMark } from "./repo-color";
import { TaskChip } from "./task-chip";

type Theme = PluginSurfaceProps["theme"];
const TABULAR = { fontVariant: ["tabular-nums"] } satisfies TextStyle;
const ROW_HEIGHT = 26;
const SUB_ROW_HEIGHT = 18;
const STRIP_HEIGHT = 46;
const AXIS_HEIGHT = 22;
const LABEL_GAP = 14;
// A duration label fits inside a bar from this width; narrower bars show only their TASK chip.
const LABEL_MIN_WIDTH = 44;
const KIND_TEXT: Record<TimelineKind, string> = {
  running: "running",
  needs: "needs input",
  done: "completed",
  failed: "failed",
  cancelled: "cancelled",
};

function hostName(label: string) {
  return label.replace(/\.local$/i, "");
}

/** Bar fill, border, and text per kind. Finished bars take the project's hue. */
function barColors(kind: TimelineKind, hue: number | undefined, theme: Theme) {
  const { colors } = theme;
  if (kind === "running")
    return {
      fill: colors.foreground,
      fillOpacity: 1,
      border: colors.foreground,
      text: colors.surface0,
    };
  if (kind === "needs")
    return {
      fill: colors.statusWarning,
      fillOpacity: 0.18,
      border: colors.statusWarning,
      text: colors.foreground,
    };
  if (kind === "failed")
    return {
      fill: colors.statusDanger,
      fillOpacity: 0.12,
      border: colors.statusDanger,
      text: colors.statusDanger,
    };
  if (kind === "cancelled")
    return {
      fill: colors.surface2,
      fillOpacity: 1,
      border: colors.foregroundMuted,
      text: colors.foregroundMuted,
    };
  if (hue === undefined)
    return {
      fill: colors.surface2,
      fillOpacity: 1,
      border: colors.border,
      text: colors.foreground,
    };
  return {
    fill: `hsl(${hue}, 38%, 58%)`,
    fillOpacity: 0.28,
    border: `hsl(${hue}, 30%, 70%)`,
    text: repoLabel(hue, theme),
  };
}

/** Breaks and the now line, drawn behind every lane so the folded time reads as one column. */
function LaneGuides({
  model,
  theme,
  s,
}: {
  model: TimelineModel;
  theme: Theme;
  s: (v: number) => number;
}) {
  const { colors } = theme;
  return (
    <>
      {model.breaks.map((gap) => (
        <View
          key={gap.x}
          style={{
            position: "absolute",
            top: 0,
            bottom: 0,
            left: gap.x,
            width: gap.width,
            backgroundColor: colors.surface1,
            borderLeftWidth: 1,
            borderRightWidth: 1,
            borderStyle: "dashed",
            borderColor: colors.border,
          }}
        />
      ))}
      <View
        style={{
          position: "absolute",
          top: 0,
          bottom: 0,
          left: model.nowX - s(1),
          width: s(2),
          backgroundColor: colors.statusDanger,
          zIndex: 3,
        }}
      />
    </>
  );
}

function Bar({
  bar,
  hue,
  height,
  top,
  sub,
  theme,
  scale,
  onOpen,
  onOpenTask,
}: {
  bar: TimelineBar;
  hue: number | undefined;
  height: number;
  top: number;
  sub: boolean;
  theme: Theme;
  scale: number;
  onOpen?: (agentId: string) => void;
  onOpenTask?: (bar: TimelineBar) => void;
}) {
  const s = (value: number) => value * scale;
  const tone = barColors(bar.kind, hue, theme);
  const duration = durationText(bar.end - bar.start);
  const label = bar.kind === "needs" ? `${duration} !` : duration;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${bar.title}, ${KIND_TEXT[bar.kind]}, ${clockText(bar.start)} to ${clockText(bar.end)}, ${duration}`}
      disabled={!onOpen}
      onPress={() => onOpen?.(bar.agentId)}
      style={({ pressed }) => ({
        position: "absolute",
        left: bar.x,
        top,
        width: bar.width,
        height,
        flexDirection: "row",
        alignItems: "center",
        gap: s(3),
        paddingHorizontal: bar.width >= LABEL_MIN_WIDTH ? s(4) : 0,
        overflow: "hidden",
        borderRadius: s(sub ? 3 : 4),
        borderWidth: 1,
        borderStyle: bar.kind === "cancelled" ? "dashed" : "solid",
        borderColor: tone.border,
        opacity: pressed ? 0.7 : 1,
        zIndex: 2,
      })}
    >
      <View
        style={{
          position: "absolute",
          top: 0,
          right: 0,
          bottom: 0,
          left: 0,
          backgroundColor: tone.fill,
          opacity: tone.fillOpacity,
        }}
      />
      {bar.taskId && bar.width >= LABEL_MIN_WIDTH / 2 ? (
        <TaskChip
          id={bar.taskId.replace(/^TASK-/, "")}
          theme={theme}
          scale={scale}
          compact
          onPress={onOpenTask && bar.workspaceId ? () => onOpenTask(bar) : undefined}
        />
      ) : null}
      {!sub && bar.width >= LABEL_MIN_WIDTH ? (
        <Text
          numberOfLines={1}
          style={{
            flexShrink: 1,
            color: tone.text,
            fontSize: s(10),
            lineHeight: s(14),
            fontWeight: bar.kind === "running" || bar.kind === "needs" ? "600" : "400",
            ...TABULAR,
          }}
        >
          {label}
        </Text>
      ) : null}
    </Pressable>
  );
}

function Row({
  label,
  lane,
  labelWidth,
  scale,
}: {
  label: ReactNode;
  lane: ReactNode;
  labelWidth: number;
  scale: number;
}) {
  return (
    <View style={{ flexDirection: "row", alignItems: "center", gap: LABEL_GAP * scale }}>
      <View style={{ width: labelWidth, minWidth: 0 }}>{label}</View>
      <View style={{ flex: 1, minWidth: 0 }}>{lane}</View>
    </View>
  );
}

function ThreadRows({
  row,
  model,
  hue,
  labelWidth,
  theme,
  scale,
  onOpen,
  onOpenTask,
}: {
  row: TimelineRow;
  model: TimelineModel;
  hue: number | undefined;
  labelWidth: number;
  theme: Theme;
  scale: number;
  onOpen?: (agentId: string) => void;
  onOpenTask?: (bar: TimelineBar) => void;
}) {
  const s = (value: number) => value * scale;
  const { colors } = theme;
  const mark = s(13);
  return (
    <>
      <Row
        labelWidth={labelWidth}
        scale={scale}
        label={
          <View style={{ flexDirection: "row", alignItems: "center", gap: s(6), minWidth: 0 }}>
            <View
              accessibilityElementsHidden
              importantForAccessibility="no-hide-descendants"
              style={{
                width: mark,
                height: mark,
                borderRadius: s(4),
                backgroundColor: repoMark(hue, theme),
                alignItems: "center",
                justifyContent: "center",
                flexShrink: 0,
              }}
            >
              <Text style={{ color: colors.surface1, fontSize: s(8.5), fontWeight: "700" }}>
                {Array.from(row.project.trim())[0]?.toUpperCase() ?? "?"}
              </Text>
            </View>
            <Text
              numberOfLines={1}
              style={{
                flexShrink: 1,
                color: colors.foreground,
                fontSize: s(11.5),
                lineHeight: s(16),
              }}
            >
              {row.bar.title}
            </Text>
          </View>
        }
        lane={
          <View
            style={{
              height: s(ROW_HEIGHT),
              borderTopWidth: 1,
              borderTopColor: colors.surface2,
            }}
          >
            <LaneGuides model={model} theme={theme} s={s} />
            <Bar
              bar={row.bar}
              hue={hue}
              top={s(5)}
              height={s(16)}
              sub={false}
              theme={theme}
              scale={scale}
              onOpen={onOpen}
              onOpenTask={onOpenTask}
            />
          </View>
        }
      />
      {row.subagents.length ? (
        <Row
          labelWidth={labelWidth}
          scale={scale}
          label={
            <Text
              numberOfLines={1}
              style={{
                paddingLeft: s(19),
                color: colors.foregroundMuted,
                fontSize: s(10.5),
                lineHeight: s(14),
              }}
            >
              {row.subagents.length === 1 ? "1 subagent" : `${row.subagents.length} subagents`}
            </Text>
          }
          lane={
            <View style={{ height: s(SUB_ROW_HEIGHT) }}>
              <LaneGuides model={model} theme={theme} s={s} />
              {row.subagents.map((bar) => (
                <Bar
                  key={bar.agentId}
                  bar={bar}
                  hue={hue}
                  top={s(3)}
                  height={s(12)}
                  sub
                  theme={theme}
                  scale={scale}
                  onOpen={onOpen}
                  onOpenTask={onOpenTask}
                />
              ))}
            </View>
          }
        />
      ) : null}
    </>
  );
}

type HostQuery = {
  data?: { threads: unknown[]; missingStart: number; olderDropped: boolean };
  isPending: boolean;
  isError: boolean;
};

function HostSection({
  host,
  client,
  query,
  rows,
  model,
  labelWidth,
  theme,
  scale,
  navigation,
}: {
  host: PluginHostSummary;
  client: BoardHostClient;
  query: HostQuery;
  rows: TimelineRow[];
  model: TimelineModel;
  labelWidth: number;
  theme: Theme;
  scale: number;
  navigation: PluginSurfaceProps["navigation"];
}) {
  const s = (value: number) => value * scale;
  const { colors } = theme;
  const online = host.status === "online";
  const palette = useHostSettings(projectColors, host.serverId, client, online);
  const hues = palette.values?.hues;
  const onOpen = navigation
    ? (agentId: string) =>
        navigation.openAgent({ agentId, serverId: host.serverId, anchor: "latest-prompt" })
    : undefined;
  const openWatchtower = client.openWatchtower;
  const onOpenTask = openWatchtower
    ? (bar: TimelineBar) => {
        if (bar.workspaceId) openWatchtower({ workspaceId: bar.workspaceId, task: bar.taskId });
      }
    : undefined;
  const muted = {
    color: colors.foregroundMuted,
    fontSize: s(11),
    lineHeight: s(16),
  } satisfies TextStyle;
  const meta = !online
    ? "Offline"
    : query.data
      ? rows.length === 1
        ? "1 thread"
        : `${rows.length} threads`
      : query.isError
        ? "Unavailable"
        : "Loading";
  const notes = !online
    ? ["Host offline. Reconnect to see its timeline."]
    : query.data
      ? timelineNotes(query.data, rows.length)
      : query.isError
        ? ["Timeline unavailable. Update or reload Board on this host."]
        : [];
  return (
    <View style={{ gap: s(2) }}>
      <View style={{ flexDirection: "row", alignItems: "center", gap: s(8), paddingTop: s(10) }}>
        <Text
          accessibilityRole="header"
          numberOfLines={1}
          style={{ flexShrink: 1, color: colors.foreground, fontSize: s(12), fontWeight: "700" }}
        >
          {hostName(host.label)}
        </Text>
        <Text style={{ ...muted, color: online ? colors.foregroundMuted : colors.statusWarning }}>
          {meta}
        </Text>
        {online && query.isPending ? (
          <ActivityIndicator size="small" color={colors.accent} />
        ) : null}
        <View style={{ flex: 1, height: 1, backgroundColor: colors.border }} />
      </View>
      {notes.map((note) => (
        <Text key={note} style={muted}>
          {note}
        </Text>
      ))}
      {rows.map((row) => (
        <ThreadRows
          key={row.agentId}
          row={row}
          model={model}
          hue={row.projectId ? hues?.[row.projectId] : undefined}
          labelWidth={labelWidth}
          theme={theme}
          scale={scale}
          onOpen={onOpen}
          onOpenTask={onOpenTask}
        />
      ))}
    </View>
  );
}

const LEGEND: { kind: TimelineKind; label: string }[] = [
  { kind: "running", label: "running" },
  { kind: "done", label: "done" },
  { kind: "needs", label: "needs you" },
  { kind: "failed", label: "failed" },
  { kind: "cancelled", label: "cancelled" },
];

export function TimelineView({
  hosts,
  clients,
  theme,
  compact,
  scale,
  navigation,
}: {
  hosts: readonly PluginHostSummary[];
  clients: ReadonlyMap<string, BoardHostClient>;
  theme: Theme;
  compact: boolean;
  scale: number;
  navigation: PluginSurfaceProps["navigation"];
}) {
  const s = (value: number) => value * scale;
  const { colors } = theme;
  // Each host loads its own timeline; the page lays them on one shared axis.
  const queries = useQueries({
    queries: hosts.map((host) => ({
      queryKey: ["board-timeline", host.serverId],
      queryFn: () => clients.get(host.serverId)!.rpc(timelineRpc, {}),
      enabled: host.status === "online" && clients.has(host.serverId),
      retry: false,
      refetchInterval: 15_000,
      refetchOnWindowFocus: false,
    })),
  });
  // The now line and running bars move once a minute.
  const minute = useClock((now) => Math.floor(now / 60_000));
  const [width, setWidth] = useState(0);
  const labelWidth = s(compact ? 112 : 230);
  const chartWidth = Math.max(width - labelWidth - s(LABEL_GAP), 1);
  const updatedAt = queries.map((query) => query.dataUpdatedAt).join(",");
  const model = useMemo(
    () =>
      timelineModel(
        hosts.map((host, index) => ({
          hostId: host.serverId,
          threads: queries[index]?.data?.threads ?? [],
        })),
        minute * 60_000,
        chartWidth,
      ),
    // Query objects change identity on every render; their update times say when data changed.
    [hosts, updatedAt, minute, chartWidth],
  );
  const muted = {
    color: colors.foregroundMuted,
    fontSize: s(11),
    lineHeight: s(15),
    ...TABULAR,
  } satisfies TextStyle;
  const mono = { ...muted, fontSize: s(10), fontFamily: "Menlo, monospace" } satisfies TextStyle;
  const footer = [
    `${model.threads} ${model.threads === 1 ? "thread" : "threads"}`,
    `${durationText(model.agentMinutes * 60_000)} agent time`,
    model.peak ? `peak ${model.peak.count} at once, ${model.peak.label}` : null,
    model.idleMinutes ? `idle ${durationText(model.idleMinutes * 60_000)} folded` : null,
  ].filter(Boolean);
  return (
    <View style={{ gap: s(6) }} onLayout={(event) => setWidth(event.nativeEvent.layout.width)}>
      <View style={{ flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: s(14) }}>
        <Text style={{ ...muted, fontSize: s(12) }}>Last 24 hours</Text>
        <View style={{ flex: 1 }} />
        {LEGEND.map((item) => {
          const tone = barColors(item.kind, undefined, theme);
          return (
            <View key={item.kind} style={{ flexDirection: "row", alignItems: "center", gap: s(5) }}>
              <View
                style={{
                  width: s(14),
                  height: s(8),
                  borderRadius: s(2),
                  borderWidth: 1,
                  borderStyle: item.kind === "cancelled" ? "dashed" : "solid",
                  borderColor: tone.border,
                  overflow: "hidden",
                }}
              >
                <View style={{ flex: 1, backgroundColor: tone.fill, opacity: tone.fillOpacity }} />
              </View>
              <Text style={muted}>{item.label}</Text>
            </View>
          );
        })}
      </View>
      {width ? (
        <>
          <Row
            labelWidth={labelWidth}
            scale={scale}
            label={<Text style={muted}>Agents at once</Text>}
            lane={
              <View
                accessibilityLabel={
                  model.peak
                    ? `Peak ${model.peak.count} agents at once, ${model.peak.label}`
                    : "No agents ran"
                }
                style={{
                  height: s(STRIP_HEIGHT),
                  borderBottomWidth: 1,
                  borderBottomColor: colors.border,
                }}
              >
                <LaneGuides model={model} theme={theme} s={s} />
                {model.load.map((bucket) => (
                  <View
                    key={bucket.x}
                    style={{
                      position: "absolute",
                      bottom: 0,
                      left: bucket.x + 1,
                      width: Math.max(bucket.width - 2, 1),
                      height: (bucket.count / (model.peak?.count ?? 1)) * s(STRIP_HEIGHT - 16),
                      borderTopLeftRadius: s(2),
                      borderTopRightRadius: s(2),
                      backgroundColor: bucket.running ? colors.foreground : colors.statusSuccess,
                      opacity: bucket.running ? 1 : 0.35,
                      zIndex: 2,
                    }}
                  />
                ))}
                {model.peak ? (
                  <Text style={{ ...mono, position: "absolute", left: s(2), top: 0, zIndex: 4 }}>
                    {`peak ${model.peak.count} · ${model.peak.label}`}
                  </Text>
                ) : null}
              </View>
            }
          />
          <Row
            labelWidth={labelWidth}
            scale={scale}
            label={null}
            lane={
              <View style={{ height: s(AXIS_HEIGHT) }}>
                {model.ticks.map((tick) => (
                  <Text
                    key={tick.x}
                    style={{
                      ...mono,
                      position: "absolute",
                      top: s(4),
                      left: tick.x - s(18),
                      width: s(36),
                      textAlign: "center",
                    }}
                  >
                    {tick.label}
                  </Text>
                ))}
                {model.breaks.map((gap) => (
                  <Text
                    key={gap.x}
                    accessibilityLabel={`Idle ${gap.label}, not drawn to scale`}
                    style={{
                      ...mono,
                      position: "absolute",
                      top: s(4),
                      left: gap.x,
                      width: gap.width,
                      textAlign: "center",
                      color: colors.foreground,
                    }}
                  >
                    {gap.label}
                  </Text>
                ))}
              </View>
            }
          />
          {hosts.map((host, index) => {
            const client = clients.get(host.serverId);
            if (!client) return null;
            return (
              <HostSection
                key={host.serverId}
                host={host}
                client={client}
                query={queries[index]}
                rows={model.rows.get(host.serverId) ?? []}
                model={model}
                labelWidth={labelWidth}
                theme={theme}
                scale={scale}
                navigation={navigation}
              />
            );
          })}
          <Text
            style={{
              ...muted,
              paddingTop: s(8),
              marginTop: s(6),
              borderTopWidth: 1,
              borderTopColor: colors.surface2,
            }}
          >
            {footer.join(" · ")}
          </Text>
        </>
      ) : null}
    </View>
  );
}
