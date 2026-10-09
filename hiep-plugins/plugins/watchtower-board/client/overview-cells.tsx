import type { PluginTheme } from "@getpaseo/plugin";
import { copyText, Icon } from "@getpaseo/plugin/client/react-native";
import { type ReactNode, useState } from "react";
import { Platform, Pressable, Text, View } from "react-native";
import type { Decision, History, ManualCheck, OverviewRun } from "../shared/overview";
import type { NeedsYouRow, Phase, PhaseName } from "./lifecycle";
import { runningLogLine } from "./run-log";

// Presentational parts of the overview. They take data through props and never fetch.
export const LIST_LIMIT = 5;
const MONO = Platform.select({
  ios: "Menlo",
  android: "monospace",
  default: "ui-monospace, Menlo, monospace",
});

type ThemeProps = { theme: PluginTheme };

export interface StatusCounts {
  done: number;
  running: number;
  blocked: number;
  todo: number;
}

function More({ count, theme }: { count: number } & ThemeProps) {
  if (count <= 0) return null;
  return (
    <Text style={{ fontSize: 11.5, color: theme.colors.foregroundMuted, marginTop: 6 }}>
      {count} more
    </Text>
  );
}

function Empty({ text, theme }: { text: string } & ThemeProps) {
  return <Text style={{ fontSize: 12.5, color: theme.colors.foregroundMuted }}>{text}</Text>;
}

/** A bordered cell with a title row. `tone` tints a cell that needs attention. */
export function OverviewCell({
  title,
  aside,
  tone,
  children,
  theme,
}: { title: string; aside?: string; tone?: "warning"; children: ReactNode } & ThemeProps) {
  const { colors } = theme;
  return (
    <View
      style={{
        // Fills its grid column, so cards in one row end on the same line.
        flexGrow: 1,
        borderWidth: 1,
        borderColor: tone === "warning" ? colors.statusWarning : colors.border,
        borderRadius: 12,
        padding: 14,
        backgroundColor: colors.surface0,
        overflow: "hidden",
      }}
    >
      {tone === "warning" ? (
        <View
          pointerEvents="none"
          style={{
            position: "absolute",
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
            backgroundColor: colors.statusWarning,
            opacity: 0.05,
          }}
        />
      ) : null}
      <View
        style={{
          flexDirection: "row",
          justifyContent: "space-between",
          alignItems: "baseline",
          marginBottom: 10,
          gap: 8,
        }}
      >
        <Text style={{ fontSize: 12.5, fontWeight: "600", color: colors.foreground }}>{title}</Text>
        {aside ? (
          <Text
            numberOfLines={1}
            style={{ fontSize: 12, color: colors.foregroundMuted, flexShrink: 1 }}
          >
            {aside}
          </Text>
        ) : null}
      </View>
      {children}
    </View>
  );
}

const phaseIcons: Record<PhaseName, string> = {
  Plan: "FileText",
  Implement: "Play",
  Verify: "ShieldCheck",
  Review: "GitPullRequest",
  Archive: "Archive",
};

function Segments({ counts, theme }: { counts: StatusCounts } & ThemeProps) {
  const { colors } = theme;
  const parts = [
    { key: "done", value: counts.done, color: colors.statusSuccess, opacity: 1 },
    { key: "running", value: counts.running, color: colors.accent, opacity: 1 },
    { key: "blocked", value: counts.blocked, color: colors.statusDanger, opacity: 1 },
    { key: "todo", value: counts.todo, color: colors.foregroundMuted, opacity: 0.25 },
  ].filter((part) => part.value > 0);
  return (
    <View style={{ flexDirection: "row", gap: 2, height: 4, marginTop: 8 }}>
      {parts.map((part) => (
        <View
          key={part.key}
          style={{
            flex: part.value,
            borderRadius: 2,
            backgroundColor: part.color,
            opacity: part.opacity,
          }}
        />
      ))}
    </View>
  );
}

export function LifecycleStepper({
  phases,
  counts,
  theme,
}: { phases: readonly Phase[]; counts: StatusCounts } & ThemeProps) {
  const { colors } = theme;
  return (
    <View
      accessibilityRole="summary"
      style={{
        flexDirection: "row",
        borderWidth: 1,
        borderColor: colors.border,
        borderRadius: 10,
        overflow: "hidden",
      }}
    >
      {phases.map((phase, index) => {
        const current = phase.state === "current";
        const color =
          phase.state === "done"
            ? colors.statusSuccess
            : current
              ? colors.accent
              : colors.foregroundMuted;
        return (
          <View
            key={phase.name}
            accessibilityLabel={`${phase.name}, ${phase.state}, ${phase.meta}`}
            style={{
              flex: 1,
              minWidth: 0,
              paddingHorizontal: 14,
              paddingTop: 11,
              paddingBottom: 12,
              borderLeftWidth: index === 0 ? 0 : 1,
              borderLeftColor: colors.border,
            }}
          >
            {current ? (
              <>
                <View
                  pointerEvents="none"
                  style={{
                    position: "absolute",
                    top: 0,
                    right: 0,
                    bottom: 0,
                    left: 0,
                    backgroundColor: colors.accent,
                    opacity: 0.06,
                  }}
                />
                <View
                  style={{
                    position: "absolute",
                    right: 0,
                    bottom: 0,
                    left: 0,
                    height: 2,
                    backgroundColor: colors.accent,
                  }}
                />
              </>
            ) : null}
            <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
              <Icon
                name={phase.state === "done" ? "CircleCheck" : phaseIcons[phase.name]}
                size={14}
                color={color}
              />
              <Text style={{ fontSize: 13, fontWeight: "600", color: colors.foreground }}>
                {phase.name}
              </Text>
            </View>
            <Text
              numberOfLines={1}
              style={{ fontSize: 11.5, color: colors.foregroundMuted, marginTop: 3 }}
            >
              {phase.meta}
            </Text>
            {current && phase.name === "Implement" ? (
              <Segments counts={counts} theme={theme} />
            ) : null}
          </View>
        );
      })}
    </View>
  );
}

function rowIcon(row: NeedsYouRow, theme: PluginTheme): { name: string; color: string } {
  const { colors } = theme;
  if (row.tone === "danger") return { name: "CircleAlert", color: colors.statusDanger };
  if (row.kind === "adr") return { name: "Scale", color: colors.foregroundMuted };
  if (row.kind === "checks") return { name: "ClipboardCheck", color: colors.foregroundMuted };
  return {
    name: "MessageCircleQuestion",
    color: row.tone === "warning" ? colors.statusWarning : colors.foregroundMuted,
  };
}

// Read-only (Q-001): each row copies its text so the owner can paste it into the composer.
export function NeedsYouCell({ rows, theme }: { rows: readonly NeedsYouRow[] } & ThemeProps) {
  const { colors } = theme;
  const [copied, setCopied] = useState<string | null>(null);
  const shown = rows.slice(0, LIST_LIMIT);
  return (
    <OverviewCell
      title="Needs you"
      aside={rows.length ? String(rows.length) : undefined}
      tone={rows.some((row) => row.tone === "danger") ? "warning" : undefined}
      theme={theme}
    >
      {shown.length === 0 ? <Empty text="Nothing waits on you." theme={theme} /> : null}
      {shown.map((row, index) => {
        const icon = rowIcon(row, theme);
        return (
          <View
            key={`${row.kind}-${row.id}`}
            style={{
              flexDirection: "row",
              alignItems: "flex-start",
              gap: 8,
              paddingVertical: 7,
              borderTopWidth: index === 0 ? 0 : 1,
              borderTopColor: colors.border,
            }}
          >
            <View style={{ marginTop: 2 }}>
              <Icon name={icon.name} size={14} color={icon.color} />
            </View>
            <View style={{ flex: 1, minWidth: 0 }}>
              <Text
                numberOfLines={2}
                style={{ fontSize: 12.5, lineHeight: 17, color: colors.foreground }}
              >
                {row.title}
              </Text>
              <Text
                numberOfLines={1}
                style={{ fontSize: 11, color: colors.foregroundMuted, marginTop: 1 }}
              >
                {row.meta}
              </Text>
            </View>
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={`Copy ${row.id}`}
              hitSlop={6}
              onPress={() => {
                void copyText(`${row.meta}: ${row.title}`).then(() => setCopied(row.id));
              }}
              style={{ paddingHorizontal: 4, paddingVertical: 2 }}
            >
              {copied === row.id ? (
                <Text style={{ fontSize: 11.5, color: colors.statusSuccess }}>Copied</Text>
              ) : (
                <Icon name="Copy" size={13} color={colors.foregroundMuted} />
              )}
            </Pressable>
          </View>
        );
      })}
      <More count={rows.length - shown.length} theme={theme} />
    </OverviewCell>
  );
}

export interface RunningTask {
  id: string;
  minutes: number;
}

const logColumns = [44, 44, 36] as const;

function LogLine({
  cells,
  tail,
  strong,
  tone,
  theme,
}: {
  cells: readonly [string, string, string, string];
  tail: string;
  strong?: boolean;
  tone?: "active" | "warning";
} & ThemeProps) {
  const { colors } = theme;
  const text = { fontFamily: MONO, fontSize: 11, lineHeight: 19 };
  const base = strong ? colors.foreground : colors.foregroundMuted;
  const resultColor =
    tone === "active" ? colors.accent : tone === "warning" ? colors.statusWarning : base;
  return (
    <View style={{ flexDirection: "row", alignItems: "center" }}>
      {logColumns.map((width, index) => (
        <Text key={width + index} style={{ ...text, width, color: base }}>
          {cells[index]}
        </Text>
      ))}
      <Text style={{ ...text, flex: 1, color: resultColor }}>{cells[3]}</Text>
      <Text style={{ ...text, color: tail.startsWith("#") ? colors.accent : base }}>{tail}</Text>
    </View>
  );
}

export function RunLogCell({
  run,
  running,
  theme,
}: { run: OverviewRun | null; running: readonly RunningTask[] } & ThemeProps) {
  const log = run?.log ?? [];
  const live = running.slice(0, LIST_LIMIT);
  const recent = log.slice(Math.max(0, log.length - (LIST_LIMIT - live.length)));
  return (
    <OverviewCell
      title="Run log"
      aside={run ? `${log.length} iterations` : undefined}
      theme={theme}
    >
      {!run ? <Empty text="No run yet." theme={theme} /> : null}
      {recent.map((row, index) => (
        <LogLine
          key={`${row.task}-${row.start}-${index}`}
          cells={[row.start, row.end, row.task.replace(/^TASK-/, ""), row.result.toLowerCase()]}
          tail={row.detail}
          theme={theme}
        />
      ))}
      {live.map((task) => {
        const line = runningLogLine(task, Boolean(run?.stopped));
        return (
          <LogLine
            key={`running-${task.id}`}
            cells={line.cells}
            tail={line.tail}
            strong
            tone={line.tone}
            theme={theme}
          />
        );
      })}
      {run && log.length === 0 && live.length === 0 ? (
        <Empty text="No iterations logged yet." theme={theme} />
      ) : null}
    </OverviewCell>
  );
}

export function ManualChecksCell({
  checks,
  theme,
}: { checks: readonly ManualCheck[] } & ThemeProps) {
  const { colors } = theme;
  const shown = checks.slice(0, LIST_LIMIT);
  return (
    <OverviewCell
      title="Manual checks"
      aside={checks.length ? `${checks.length} pending` : undefined}
      theme={theme}
    >
      {shown.length === 0 ? <Empty text="No manual checks pending." theme={theme} /> : null}
      {shown.map((check, index) => (
        <View
          key={`${index}-${check.text}`}
          style={{
            flexDirection: "row",
            alignItems: "flex-start",
            gap: 8,
            paddingVertical: 7,
            borderTopWidth: index === 0 ? 0 : 1,
            borderTopColor: colors.border,
          }}
        >
          <View
            style={{
              width: 13,
              height: 13,
              marginTop: 2,
              borderRadius: 4,
              borderWidth: 1.5,
              borderColor: colors.foregroundMuted,
              opacity: 0.6,
            }}
          />
          <View style={{ flex: 1, minWidth: 0 }}>
            <Text
              numberOfLines={2}
              style={{ fontSize: 12.5, lineHeight: 17, color: colors.foreground }}
            >
              {check.text}
            </Text>
            {check.tasks.length ? (
              <Text style={{ fontSize: 11, color: colors.foregroundMuted, marginTop: 1 }}>
                {check.tasks.join(", ")}
              </Text>
            ) : null}
          </View>
        </View>
      ))}
      <More count={checks.length - shown.length} theme={theme} />
    </OverviewCell>
  );
}

const HISTORY_ROWS = 3;

function HistoryRow({
  icon,
  iconColor,
  title,
  meta,
  first,
  theme,
}: { icon: string; iconColor: string; title: string; meta: string; first: boolean } & ThemeProps) {
  const { colors } = theme;
  return (
    <View
      style={{
        flexDirection: "row",
        alignItems: "flex-start",
        gap: 8,
        paddingVertical: 7,
        borderTopWidth: first ? 0 : 1,
        borderTopColor: colors.border,
      }}
    >
      <View style={{ marginTop: 2 }}>
        <Icon name={icon} size={14} color={iconColor} />
      </View>
      <View style={{ flex: 1, minWidth: 0 }}>
        <Text numberOfLines={1} style={{ fontSize: 12.5, color: colors.foreground }}>
          {title}
        </Text>
        <Text
          numberOfLines={1}
          style={{ fontSize: 11, color: colors.foregroundMuted, marginTop: 1 }}
        >
          {meta}
        </Text>
      </View>
    </View>
  );
}

// Proposed ADRs come first, because the owner still has to accept or reject them.
export function HistoryCell({
  decisions,
  history,
  theme,
}: { decisions: readonly Decision[]; history: readonly History[] } & ThemeProps) {
  const { colors } = theme;
  const ordered = [...decisions].sort(
    (a, b) =>
      Number(b.status.toLowerCase() === "proposed") -
        Number(a.status.toLowerCase() === "proposed") || b.id.localeCompare(a.id),
  );
  const adrs = ordered.slice(0, HISTORY_ROWS);
  const plans = history.slice(0, HISTORY_ROWS);
  const hidden = ordered.length - adrs.length + (history.length - plans.length);
  return (
    <OverviewCell title="Decisions and history" theme={theme}>
      {adrs.length === 0 && plans.length === 0 ? (
        <Empty text="No decisions or archived plans yet." theme={theme} />
      ) : null}
      {adrs.map((decision, index) => (
        <HistoryRow
          key={decision.id}
          icon="Scale"
          iconColor={
            decision.status.toLowerCase() === "proposed"
              ? colors.statusWarning
              : colors.foregroundMuted
          }
          title={decision.title}
          meta={`${decision.id}, ${decision.status}`}
          first={index === 0}
          theme={theme}
        />
      ))}
      {plans.map((plan, index) => (
        <HistoryRow
          key={plan.slug}
          icon="Archive"
          iconColor={colors.foregroundMuted}
          title={plan.title}
          meta={[plan.date ? `Archived ${plan.date}` : plan.slug, plan.hasLearn ? "LEARN.md" : null]
            .filter(Boolean)
            .join(", ")}
          first={adrs.length === 0 && index === 0}
          theme={theme}
        />
      ))}
      <More count={hidden} theme={theme} />
    </OverviewCell>
  );
}
