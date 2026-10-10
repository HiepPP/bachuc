import type { PluginTheme } from "@getpaseo/plugin";
import { Icon } from "@getpaseo/plugin/client/react-native";
import { type ReactNode, useState } from "react";
import { Platform, Pressable, Text, View } from "react-native";
import type { Decision, History, ManualCheck } from "../shared/overview";
import { DECISION_ID } from "../shared/overview";
import type { Phase, PhaseName } from "./lifecycle";

// Presentational parts of the overview. They take data through props and never fetch.
export const LIST_LIMIT = 5;
export const MONO = Platform.select({
  ios: "Menlo",
  android: "monospace",
  default: "ui-monospace, Menlo, monospace",
});

export type ThemeProps = { theme: PluginTheme };

export interface StatusCounts {
  done: number;
  running: number;
  blocked: number;
  todo: number;
}

/** Shows the first `LIST_LIMIT` items until the owner expands the list. */
export function useLimit<Item>(items: readonly Item[]) {
  const [expanded, setExpanded] = useState(false);
  return {
    shown: expanded ? items : items.slice(0, LIST_LIMIT),
    more: { total: items.length, expanded, onToggle: () => setExpanded((value) => !value) },
  };
}

export function ShowMore({
  total,
  expanded,
  onToggle,
  theme,
}: { total: number; expanded: boolean; onToggle(): void } & ThemeProps) {
  if (total <= LIST_LIMIT) return null;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ expanded }}
      onPress={onToggle}
      style={({ pressed }) => ({
        alignSelf: "flex-start",
        paddingVertical: 4,
        marginTop: 4,
        opacity: pressed ? 0.6 : 1,
      })}
    >
      <Text style={{ fontSize: 11.5, fontWeight: "500", color: theme.colors.accent }}>
        {expanded ? "Show less" : `Show ${total - LIST_LIMIT} more`}
      </Text>
    </Pressable>
  );
}

export function Empty({ text, theme }: { text: string } & ThemeProps) {
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
  compact = false,
  theme,
}: { phases: readonly Phase[]; counts: StatusCounts; compact?: boolean } & ThemeProps) {
  const { colors } = theme;
  // A narrow panel gives each step about 60 px: names only, with no icon or meta line.
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
              paddingHorizontal: compact ? 4 : 14,
              paddingTop: compact ? 8 : 11,
              paddingBottom: compact ? 9 : 12,
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
            {compact ? (
              <Text
                numberOfLines={1}
                style={{
                  fontSize: 11.5,
                  fontWeight: current ? "700" : "500",
                  textAlign: "center",
                  color: current ? colors.foreground : color,
                }}
              >
                {phase.name}
              </Text>
            ) : (
              <>
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
              </>
            )}
            {current && phase.name === "Implement" && !compact ? (
              <Segments counts={counts} theme={theme} />
            ) : null}
          </View>
        );
      })}
    </View>
  );
}

export function ManualChecksCell({
  checks,
  theme,
}: { checks: readonly ManualCheck[] } & ThemeProps) {
  const { colors } = theme;
  const { shown, more } = useLimit(checks);
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
      <ShowMore {...more} theme={theme} />
    </OverviewCell>
  );
}

function HistoryRow({
  icon,
  iconColor,
  title,
  meta,
  first,
  onPress,
  label,
  theme,
}: {
  icon: string;
  iconColor: string;
  title: string;
  meta: string;
  first: boolean;
  // Makes the row a button.
  onPress?(): void;
  label?: string;
} & ThemeProps) {
  const { colors } = theme;
  const content = (
    <>
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
    </>
  );
  const row = {
    flexDirection: "row",
    alignItems: "flex-start",
    gap: 8,
    paddingVertical: 7,
    borderTopWidth: first ? 0 : 1,
    borderTopColor: colors.border,
  } as const;
  if (!onPress) return <View style={row}>{content}</View>;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      onPress={onPress}
      style={({ pressed }) => ({ ...row, opacity: pressed ? 0.6 : 1 })}
    >
      {content}
    </Pressable>
  );
}

// Proposed ADRs come first, because the owner still has to accept or reject them.
export function DecisionsCell({
  decisions,
  onOpen,
  theme,
}: {
  decisions: readonly Decision[];
  onOpen(decision: { id: string; title: string }): void;
} & ThemeProps) {
  const { colors } = theme;
  const proposed = (decision: Decision) => decision.status.toLowerCase() === "proposed";
  const ordered = [...decisions].sort(
    (a, b) => Number(proposed(b)) - Number(proposed(a)) || b.id.localeCompare(a.id),
  );
  const { shown, more } = useLimit(ordered);
  return (
    <OverviewCell title="Decisions" theme={theme}>
      {shown.length === 0 ? <Empty text="No decisions yet." theme={theme} /> : null}
      {shown.map((decision, index) => (
        <HistoryRow
          key={decision.id}
          icon="Scale"
          iconColor={proposed(decision) ? colors.statusWarning : colors.foregroundMuted}
          title={decision.title}
          meta={[decision.id, decision.status, decision.date].filter(Boolean).join(", ")}
          first={index === 0}
          label={`Open ${decision.id} ${decision.title}`}
          onPress={
            DECISION_ID.test(decision.id)
              ? () => onOpen({ id: decision.id, title: decision.title })
              : undefined
          }
          theme={theme}
        />
      ))}
      <ShowMore {...more} theme={theme} />
    </OverviewCell>
  );
}

export function ArchivedPlansCell({
  history,
  theme,
}: { history: readonly History[] } & ThemeProps) {
  const { colors } = theme;
  const { shown, more } = useLimit(history);
  return (
    <OverviewCell title="Archived plans" theme={theme}>
      {shown.length === 0 ? <Empty text="No archived plans yet." theme={theme} /> : null}
      {shown.map((plan, index) => (
        <HistoryRow
          key={plan.slug}
          icon="Archive"
          iconColor={colors.foregroundMuted}
          title={plan.title}
          meta={[plan.date ? `Archived ${plan.date}` : plan.slug, plan.hasLearn ? "LEARN.md" : null]
            .filter(Boolean)
            .join(", ")}
          first={index === 0}
          theme={theme}
        />
      ))}
      <ShowMore {...more} theme={theme} />
    </OverviewCell>
  );
}
