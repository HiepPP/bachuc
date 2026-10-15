import { type PluginHostProps, type PluginSurfaceProps, useRpc } from "@getpaseo/plugin/client";
import { Icon, ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { type ReactNode, useCallback, useMemo, useRef, useState } from "react";
import { Pressable, type ScrollView as NativeScrollView, Text, View } from "react-native";
import { readBoardRpc } from "../shared/board";
import { readOverviewRpc } from "../shared/overview";
import { useAgentSender } from "./agent-target";
import { matchBuilders } from "./builders";
import { dashboardSummary, withLiveStatus } from "./dashboard";
import { type DecisionTarget, DecisionModal } from "./decision-modal";
import { lifecyclePhases, needsYou } from "./lifecycle";
import { NeedsYouCell } from "./needs-you";
import {
  ArchivedPlansCell,
  DecisionsCell,
  LifecycleStepper,
  ManualChecksCell,
  MONO,
  OverviewCell,
  type ThemeProps,
} from "./overview-cells";
import { AutorunLogCard, ProgressCard, TasksCard, WaitingCard } from "./board-rail";
import { buildPlanTree, type PlanTree as PlanTreeModel } from "./plan-tree";
import { commitShas, PlanTree } from "./plan-tree-view";
import { AutorunStrip, clockAt } from "./run-cells";
import { useRunAgents } from "./run-agents";
import { buildTimeline } from "./timeline";

// From STACK up, the page has a fixed left rail with the former Explorer board (progress, Waiting on
// the owner, tasks, and the Autorun log) and the plan tree on the right. Below STACK everything
// stacks in one scroll. The right side lays itself out by its own width: two columns under the tree
// from WIDE_GRID, and below NARROW the tree drops its spine.
const STACK = 720;
const RAIL_WIDTH = 360;
const WIDE_GRID = 900;
const NARROW = 520;
// Below this width the five lifecycle steps cannot fit an icon, name, and meta line each.
const COMPACT_STEPPER = 720;
const GAP = 14;

type WatchtowerOverviewProps = PluginHostProps & {
  workspaceId: string;
  projectName: string | null;
  // A TASK the opener asked to show selected in the task list.
  initialTaskId?: string;
  // The page's navigation; Needs you uses it to open the answer target's thread.
  navigation?: PluginSurfaceProps["navigation"];
};

function Row({ children }: { children: ReactNode }) {
  return <View style={{ flexDirection: "row", gap: GAP, alignItems: "stretch" }}>{children}</View>;
}

function Column({ flex = 1, children }: { flex?: number; children: ReactNode }) {
  return <View style={{ flex, minWidth: 0 }}>{children}</View>;
}

// Four numbers of mock D v2. A number with no data is left out, and with none the line is gone.
function PaceLine({
  pace,
  hasAverage,
  theme,
}: { pace: PlanTreeModel["pace"]; hasAverage: boolean } & ThemeProps) {
  const { colors } = theme;
  const items = [
    hasAverage ? { value: `${pace.averageMinutes}m`, label: "average task" } : null,
    pace.finishClock ? { value: `~${pace.finishClock}`, label: "estimated finish" } : null,
    pace.maxParallel > 0 ? { value: String(pace.maxParallel), label: "builders at once" } : null,
    pace.slowest
      ? { value: `${pace.slowest.id} +${pace.slowest.minutes}m`, label: "slowest over average" }
      : null,
  ].filter((item) => item !== null);
  if (!items.length) return null;
  return (
    <View
      accessibilityRole="summary"
      style={{ flexDirection: "row", flexWrap: "wrap", columnGap: 24, rowGap: 8, marginBottom: 12 }}
    >
      {items.map((item) => (
        <View key={item.label}>
          <Text
            style={{ fontFamily: MONO, fontSize: 15, fontWeight: "600", color: colors.foreground }}
          >
            {item.value}
          </Text>
          <Text style={{ fontSize: 11, marginTop: 1, color: colors.foregroundMuted }}>
            {item.label}
          </Text>
        </View>
      ))}
    </View>
  );
}

export function WatchtowerOverview({
  workspaceId,
  projectName,
  host,
  theme,
  initialTaskId,
  navigation,
}: WatchtowerOverviewProps) {
  const { colors } = theme;
  const read = useRpc(readOverviewRpc);
  const overview = useQuery({
    queryKey: ["watchtower-overview", host.id, workspaceId],
    queryFn: () => read({ workspaceId }),
    retry: false,
  });
  // Only the board carries task briefs; the pill reads the same query.
  const readTasks = useRpc(readBoardRpc);
  const board = useQuery({
    queryKey: ["watchtower-board", host.id, workspaceId],
    queryFn: () => readTasks({ workspaceId }),
    retry: false,
  });
  const [width, setWidth] = useState(0);
  const stacked = width > 0 && width < STACK;
  // The width of the timeline-first side.
  const mainWidth = stacked ? width : width - RAIL_WIDTH;
  const narrow = mainWidth > 0 && mainWidth < NARROW;
  const data = overview.data;
  const runAgents = useRunAgents(host.id, workspaceId);
  const builders = useMemo(() => matchBuilders(runAgents, data?.tasks ?? []), [runAgents, data]);
  // A string key, so the 15 s agent refresh rebuilds the tree only when the set changes.
  const liveKey = Object.entries(builders.byTask)
    .filter(([, builder]) => builder.state === "running" || builder.state === "waiting")
    .map(([taskId]) => taskId)
    .sort()
    .join(",");
  const live = useMemo(() => new Set(liveKey ? liveKey.split(",") : []), [liveKey]);
  // The rail reads the same live status as the plan tree.
  const liveBoard = useMemo(
    () => board.data && { ...board.data, tasks: withLiveStatus(board.data.tasks, live) },
    [board.data, live],
  );
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
    const { counts } = dashboardSummary(withLiveStatus(data.tasks, live));
    const built = buildPlanTree(data, started, live);
    // Without RUN.md there are no logged times, so any time text would be a guess from minute 0.
    const tree: PlanTreeModel = data.run
      ? built
      : {
          ...built,
          waves: built.waves.map((wave) => ({
            ...wave,
            rows: wave.rows.map((row) => ({ ...row, time: null, over: false })),
          })),
        };
    return {
      timeline,
      started,
      tree,
      classes: Object.fromEntries(data.tasks.map((task) => [task.id, task.taskClass])),
      commits: commitShas(data.run?.log ?? []),
      rows,
      phases: lifecyclePhases(data),
      counts: {
        done: counts.done,
        running: counts.active,
        blocked: counts.blocked,
        todo: counts.todo,
      },
    };
  }, [data, live]);
  // The ADR the owner is reading, if any.
  const [decision, setDecision] = useState<DecisionTarget | null>(null);
  const agent = useAgentSender(host.id, workspaceId, (derived?.rows.length ?? 0) > 0);
  // The TASK selected in the task list. The opener starts it, a row of the tree or the list sets it.
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(initialTaskId ?? null);
  // Only the stacked page scrolls as one; the rail of the wide page has its own scroll view.
  const stackScroll = useRef<NativeScrollView>(null);
  const revealTask = useCallback((row: View) => {
    const scroll = stackScroll.current;
    if (!scroll) return;
    row.measureLayout(
      scroll.getInnerViewNode(),
      (_x, y) => scroll.scrollTo({ y: Math.max(0, y - 12), animated: true }),
      () => undefined,
    );
  }, []);
  const openAgent = navigation
    ? (agentId: string) => navigation.openAgent({ agentId, serverId: host.id })
    : undefined;

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
        onPress={() => {
          void overview.refetch();
          void board.refetch();
        }}
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
      onOpenAgent={openAgent}
      onOpenDecision={setDecision}
      theme={theme}
    />
  ) : null;
  const decisions = data ? (
    <DecisionsCell decisions={data.decisions} onOpen={setDecision} theme={theme} />
  ) : null;
  const archived = data ? <ArchivedPlansCell history={data.history} theme={theme} /> : null;
  // The rail: the former Explorer board. Its log lists every RUN.md row, newest first.
  const progress = liveBoard ? <ProgressCard tasks={liveBoard.tasks} theme={theme} /> : null;
  const waiting = board.data ? (
    <WaitingCard
      questions={board.data.questions}
      proposedAdrs={board.data.proposedAdrs}
      theme={theme}
    />
  ) : null;
  const tasks = (
    <TasksCard
      board={liveBoard}
      error={board.error}
      workspaceId={workspaceId}
      sourceKey={workspaceId}
      selectedTaskId={selectedTaskId}
      onSelectTask={setSelectedTaskId}
      onRevealSelected={stacked ? revealTask : undefined}
      theme={theme}
    />
  );
  const logRows = useMemo(() => [...(data?.run?.log ?? [])].reverse(), [data]);
  const autorunLog = <AutorunLogCard run={board.data?.run ?? null} rows={logRows} theme={theme} />;

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
          {placeholder("tree", 240)}
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
    const { timeline, started, tree } = derived;
    const stoppedAt = data.run?.stopped ? clockAt(started, data.run.nowMinute ?? 0) : null;
    const since = clockAt(started, 0);
    // The run state from the autorun log; the lead row shows it when no lead agent is found.
    const runState =
      data.run === null
        ? "No autorun yet"
        : stoppedAt
          ? `Run stopped at ${stoppedAt}`
          : data.run.finished
            ? "Run finished"
            : since
              ? `Autorun since ${since}`
              : "Autorun running";
    const treeCell = (
      <OverviewCell title="Plan" aside={runState} theme={theme}>
        {data.run ? (
          <PaceLine
            // A stopped run has no real "now", so its finish estimate would be in the past.
            pace={data.run.stopped ? { ...tree.pace, finishClock: null } : tree.pace}
            hasAverage={timeline.bars.some((bar) => bar.kind === "done")}
            theme={theme}
          />
        ) : null}
        <PlanTree
          tree={tree}
          builders={builders.byTask}
          lead={builders.lead}
          narrow={narrow}
          theme={theme}
          onSelectTask={setSelectedTaskId}
          onOpenAgent={openAgent}
          planTitle={data.plan.title}
          runState={runState}
          classes={derived.classes}
          commits={derived.commits}
        />
      </OverviewCell>
    );
    const checks = <ManualChecksCell checks={data.manualChecks} theme={theme} />;
    body = (
      <View style={{ gap: GAP }}>
        <LifecycleStepper
          phases={derived.phases}
          counts={derived.counts}
          compact={mainWidth > 0 && mainWidth < COMPACT_STEPPER}
          theme={theme}
        />
        {data.warnings.map((warning) => (
          <Text key={warning} style={{ fontSize: 12, color: colors.statusWarning }}>
            {warning}
          </Text>
        ))}
        <AutorunStrip run={data.run} theme={theme} />
        {mainWidth >= WIDE_GRID ? (
          <>
            {treeCell}
            <Row>
              <Column>{needs}</Column>
              <Column>
                <View style={{ gap: GAP }}>
                  {checks}
                  {decisions}
                  {archived}
                </View>
              </Column>
            </Row>
          </>
        ) : narrow ? (
          <>
            {treeCell}
            {needs}
            {checks}
            {decisions}
            {archived}
          </>
        ) : (
          <>
            {treeCell}
            <Row>
              <Column>{needs}</Column>
              <Column>{checks}</Column>
            </Row>
            <Row>
              <Column>{decisions}</Column>
              <Column>{archived}</Column>
            </Row>
          </>
        )}
      </View>
    );
  }

  const pad = stacked ? 12 : 24;
  return (
    <View
      style={{ flex: 1, backgroundColor: colors.surface0 }}
      // The outer width does not depend on the padding, so the layout cannot flip back and forth.
      onLayout={(event) => setWidth(event.nativeEvent.layout.width)}
    >
      {stacked ? (
        <ScrollView
          ref={stackScroll}
          style={{ flex: 1 }}
          contentContainerStyle={{ padding: pad, gap: GAP }}
        >
          {header}
          {progress}
          {waiting}
          {body}
          {tasks}
          {autorunLog}
        </ScrollView>
      ) : (
        <View style={{ flex: 1, flexDirection: "row" }}>
          <ScrollView
            style={{
              width: RAIL_WIDTH,
              flexGrow: 0,
              flexShrink: 0,
              borderRightWidth: 1,
              borderRightColor: colors.border,
              backgroundColor: colors.surface1,
            }}
            contentContainerStyle={{ padding: 16, gap: GAP }}
          >
            {progress}
            {waiting}
            {tasks}
            {autorunLog}
          </ScrollView>
          <ScrollView style={{ flex: 1 }} contentContainerStyle={{ padding: pad, paddingTop: 18 }}>
            {header}
            {body}
          </ScrollView>
        </View>
      )}
      <DecisionModal
        target={decision}
        workspaceId={workspaceId}
        host={host}
        theme={theme}
        onClose={() => setDecision(null)}
      />
    </View>
  );
}
