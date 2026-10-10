import { type PluginHostProps, useRpc } from "@getpaseo/plugin/client";
import { Icon, ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { type ReactNode, useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { readOverviewRpc } from "../shared/overview";
import { useAgentSender } from "./agent-target";
import { dashboardSummary } from "./dashboard";
import { type DecisionTarget, DecisionModal } from "./decision-modal";
import { lifecyclePhases, needsYou } from "./lifecycle";
import { NeedsYouCell } from "./needs-you";
import {
  ArchivedPlansCell,
  DecisionsCell,
  LifecycleStepper,
  ManualChecksCell,
  OverviewCell,
  RunLogCell,
} from "./overview-cells";
import { buildTimeline } from "./timeline";
import { clockAt } from "./timeline-geometry";
import { BranchTimeline } from "./timeline-view";

// Below this width the grid drops from three columns to two.
const WIDE_GRID = 1000;
const GAP = 14;

type WatchtowerOverviewProps = PluginHostProps & {
  workspaceId: string;
  projectName: string | null;
};

function Row({ children }: { children: ReactNode }) {
  return <View style={{ flexDirection: "row", gap: GAP, alignItems: "stretch" }}>{children}</View>;
}

function Column({ flex = 1, children }: { flex?: number; children: ReactNode }) {
  return <View style={{ flex, minWidth: 0 }}>{children}</View>;
}

export function WatchtowerOverview({
  workspaceId,
  projectName,
  host,
  theme,
}: WatchtowerOverviewProps) {
  const { colors } = theme;
  const read = useRpc(readOverviewRpc);
  const overview = useQuery({
    queryKey: ["watchtower-overview", host.id, workspaceId],
    queryFn: () => read({ workspaceId }),
    retry: false,
  });
  const [width, setWidth] = useState(0);
  const data = overview.data;
  const derived = useMemo(() => {
    if (!data) return null;
    const timeline = buildTimeline(data);
    const started = data.run?.started ?? null;
    // A stopped run has no real "now", so its answer-by times would be in the past.
    const deadlines = new Map(
      data.run?.stopped ? [] : timeline.deadlines.map((entry) => [entry.questionId, entry.minute]),
    );
    const rows = needsYou(data).map((row) => {
      const minute = deadlines.get(row.id);
      const clock = minute === undefined ? null : clockAt(started, minute);
      return clock ? { ...row, meta: `${row.meta}, answer by ${clock}` } : row;
    });
    const { counts } = dashboardSummary(data.tasks);
    return {
      timeline,
      started,
      rows,
      phases: lifecyclePhases(data),
      counts: {
        done: counts.done,
        running: counts.active,
        blocked: counts.blocked,
        todo: counts.todo,
      },
      running: timeline.bars
        .filter((bar) => bar.kind === "running")
        .map((bar) => ({ id: bar.id, minutes: Math.round(bar.end - bar.start) })),
    };
  }, [data]);
  // The ADR the owner is reading, if any.
  const [decision, setDecision] = useState<DecisionTarget | null>(null);
  const agent = useAgentSender(host.id, workspaceId, (derived?.rows.length ?? 0) > 0);

  const muted = { fontSize: 12, color: colors.foregroundMuted };
  const header = (
    <View style={{ flexDirection: "row", alignItems: "flex-start", gap: 16, marginBottom: 14 }}>
      <View style={{ flex: 1, minWidth: 0 }}>
        <Text
          accessibilityRole="header"
          numberOfLines={2}
          style={{ fontSize: 18, fontWeight: "600", letterSpacing: -0.2, color: colors.foreground }}
        >
          {data && data.tasks.length > 0 ? data.plan.title : "Watchtower"}
        </Text>
        <Text numberOfLines={1} style={{ ...muted, marginTop: 2 }}>
          {[
            projectName,
            data?.plan.slug,
            data?.plan.updated ? `updated ${data.plan.updated}` : null,
          ]
            .filter(Boolean)
            .join(", ")}
        </Text>
      </View>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Refresh Watchtower overview"
        accessibilityState={{ disabled: overview.isFetching, busy: overview.isFetching }}
        disabled={overview.isFetching}
        onPress={() => void overview.refetch()}
        style={{
          width: 30,
          height: 30,
          borderRadius: 7,
          alignItems: "center",
          justifyContent: "center",
          opacity: overview.isFetching ? 0.5 : 1,
        }}
      >
        <Icon name="RefreshCw" size={15} color={colors.foregroundMuted} />
      </Pressable>
    </View>
  );

  // The cells that both layouts use. They render only once `data` has loaded.
  const needs = derived ? (
    <NeedsYouCell
      rows={derived.rows}
      target={agent.target}
      send={agent.send}
      onOpenDecision={setDecision}
      theme={theme}
    />
  ) : null;
  const decisions = data ? (
    <DecisionsCell decisions={data.decisions} onOpen={setDecision} theme={theme} />
  ) : null;
  const archived = data ? <ArchivedPlansCell history={data.history} theme={theme} /> : null;

  let body: ReactNode;
  if (overview.error) {
    body = (
      <View
        accessibilityRole="alert"
        style={{
          flexDirection: "row",
          alignItems: "center",
          gap: 12,
          padding: 14,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: colors.statusDanger,
        }}
      >
        <Icon name="CircleAlert" size={15} color={colors.statusDanger} />
        <Text style={{ flex: 1, fontSize: 13, color: colors.statusDanger }}>
          Could not load the overview. {overview.error.message}
        </Text>
        <Pressable
          accessibilityRole="button"
          onPress={() => void overview.refetch()}
          style={{
            paddingHorizontal: 10,
            paddingVertical: 6,
            borderRadius: 7,
            borderWidth: 1,
            borderColor: colors.border,
          }}
        >
          <Text style={{ fontSize: 12, fontWeight: "500", color: colors.foreground }}>Retry</Text>
        </Pressable>
      </View>
    );
  } else if (!data || !derived) {
    const placeholder = (key: string, height: number) => (
      <View
        key={key}
        style={{
          flex: 1,
          height,
          borderRadius: 12,
          borderWidth: 1,
          borderColor: colors.border,
          padding: 14,
        }}
      >
        {[0.5, 0.8, 0.65].map((fraction) => (
          <View
            key={fraction}
            style={{
              width: `${fraction * 100}%`,
              height: 8,
              borderRadius: 4,
              marginBottom: 10,
              backgroundColor: colors.foregroundMuted,
              opacity: 0.12,
            }}
          />
        ))}
      </View>
    );
    body = (
      <View accessibilityLabel="Loading Watchtower overview" style={{ gap: GAP }}>
        {placeholder("stepper", 58)}
        <Row>
          {placeholder("timeline", 240)}
          {placeholder("needs", 240)}
        </Row>
        <Row>
          {placeholder("log", 200)}
          {placeholder("checks", 200)}
        </Row>
        <Row>
          {placeholder("decisions", 200)}
          {placeholder("archive", 200)}
        </Row>
      </View>
    );
  } else if (data.tasks.length === 0 && data.message) {
    body = (
      <View style={{ gap: GAP }}>
        <View style={{ paddingVertical: 12 }}>
          <View
            style={{
              width: 32,
              height: 32,
              borderRadius: 8,
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: colors.surface2,
            }}
          >
            <Icon name="ListChecks" size={16} color={colors.foregroundMuted} />
          </View>
          <Text
            style={{ fontSize: 15, fontWeight: "600", color: colors.foreground, marginTop: 12 }}
          >
            No plan yet
          </Text>
          <Text style={{ ...muted, fontSize: 12.5, marginTop: 4, maxWidth: 520 }}>
            {data.message}
          </Text>
        </View>
        <Row>
          <Column>{needs}</Column>
          <Column>{decisions}</Column>
          <Column>{archived}</Column>
        </Row>
      </View>
    );
  } else {
    const { timeline, started } = derived;
    const finish = timeline.finishMinute === null ? null : clockAt(started, timeline.finishMinute);
    const stoppedAt = data.run?.stopped ? clockAt(started, data.run.nowMinute ?? 0) : null;
    const timelineCell = (
      <OverviewCell
        title="Branch timeline"
        aside={
          stoppedAt
            ? `Run stopped at ${stoppedAt}`
            : finish
              ? `Finish about ${finish}`
              : data.run?.finished
                ? "Run finished"
                : started
                  ? `Autorun since ${clockAt(started, 0)}`
                  : undefined
        }
        theme={theme}
      >
        <BranchTimeline
          model={timeline}
          nowMinute={data.run?.nowMinute ?? null}
          started={started}
          theme={theme}
        />
      </OverviewCell>
    );
    const log = <RunLogCell run={data.run} running={derived.running} theme={theme} />;
    const checks = <ManualChecksCell checks={data.manualChecks} theme={theme} />;
    body = (
      <View style={{ gap: GAP }}>
        <LifecycleStepper phases={derived.phases} counts={derived.counts} theme={theme} />
        {data.warnings.map((warning) => (
          <Text key={warning} style={{ fontSize: 12, color: colors.statusWarning }}>
            {warning}
          </Text>
        ))}
        {width >= WIDE_GRID ? (
          <>
            <Row>
              <Column flex={2}>{timelineCell}</Column>
              <Column>{needs}</Column>
            </Row>
            <Row>
              <Column>{log}</Column>
              <Column>{checks}</Column>
            </Row>
            <Row>
              <Column>{decisions}</Column>
              <Column>{archived}</Column>
            </Row>
          </>
        ) : (
          <>
            {timelineCell}
            <Row>
              <Column>{needs}</Column>
              <Column>{log}</Column>
            </Row>
            <Row>
              <Column>{checks}</Column>
              <Column>{archived}</Column>
            </Row>
            {decisions}
          </>
        )}
      </View>
    );
  }

  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: colors.surface0 }}
      contentContainerStyle={{ padding: 24, paddingTop: 18 }}
    >
      <View onLayout={(event) => setWidth(event.nativeEvent.layout.width)}>
        {header}
        {body}
      </View>
      <DecisionModal
        target={decision}
        workspaceId={workspaceId}
        host={host}
        theme={theme}
        onClose={() => setDecision(null)}
      />
    </ScrollView>
  );
}
