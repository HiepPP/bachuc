import type { PluginTheme } from "@getpaseo/plugin";
import { Icon, TextInput } from "@getpaseo/plugin/client/react-native";
import { useState } from "react";
import { Pressable, Text, View } from "react-native";
import { DECISION_ID } from "../shared/overview";
import {
  type AgentTarget,
  acceptDecisionPrompt,
  answerPrompt,
  defaultAnswerPrompt,
  manualChecksPrompt,
  rejectDecisionPrompt,
} from "./actions";
import type { DecisionTarget } from "./decision-modal";
import type { NeedsYouRow } from "./lifecycle";
import { Empty, OverviewCell, ShowMore, type ThemeProps, useLimit } from "./overview-cells";

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

type ButtonTone = "primary" | "danger";

function ActionButton({
  label,
  subject,
  tone,
  disabled,
  onPress,
  theme,
}: {
  label: string;
  // What the button acts on, for the accessibility label.
  subject: string;
  tone?: ButtonTone;
  disabled?: boolean;
  onPress(): void;
} & ThemeProps) {
  const { colors } = theme;
  const color =
    tone === "danger"
      ? colors.statusDanger
      : tone === "primary"
        ? colors.accent
        : colors.foreground;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${label} ${subject}`}
      accessibilityState={{ disabled: Boolean(disabled) }}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => ({
        paddingHorizontal: 9,
        paddingVertical: 4,
        borderRadius: 6,
        borderWidth: 1,
        borderColor: tone === "danger" ? colors.statusDanger : colors.border,
        opacity: disabled ? 0.4 : pressed ? 0.6 : 1,
      })}
    >
      <Text style={{ fontSize: 11.5, fontWeight: "500", color }}>{label}</Text>
    </Pressable>
  );
}

function agentNote(target: AgentTarget | null): { text: string; warning: boolean } | null {
  if (!target) return null;
  if (target.state === "none") return { text: "No agent in this workspace", warning: true };
  if (target.state === "busy") return { text: "Agent busy", warning: true };
  return { text: `Actions go to ${target.name}`, warning: false };
}

// Each action sends a Watchtower prompt to the workspace's newest idle agent, which edits the
// plan files. The board writes nothing itself.
export function NeedsYouCell({
  rows,
  target,
  send,
  onOpenAgent,
  onOpenDecision,
  theme,
}: {
  rows: readonly NeedsYouRow[];
  // Null until the first agent lookup succeeds; send looks again and reports any error.
  target: AgentTarget | null;
  send(prompt: string): Promise<boolean>;
  // Opens the answer target's thread, so the owner sees where a prompt goes (ADR-0013).
  onOpenAgent?: (agentId: string) => void;
  onOpenDecision(decision: DecisionTarget): void;
} & ThemeProps) {
  const { colors } = theme;
  const { shown, more } = useLimit(rows);
  // The one row whose full text and default show; a press on its text toggles it.
  const [expanded, setExpanded] = useState<string | null>(null);
  // The row whose answer field is open, and what the owner has typed so far.
  const [answering, setAnswering] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  // The ADR row whose Reject waits for a second press.
  const [rejecting, setRejecting] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const noAgent = target?.state === "none" || target?.state === "busy";
  const note = rows.length > 0 ? agentNote(target) : null;

  const run = async (prompt: string): Promise<boolean> => {
    setSending(true);
    try {
      return await send(prompt);
    } finally {
      setSending(false);
    }
  };
  // Props every button of a row shares. A button that does not send passes `disabled` itself.
  const button = (row: NeedsYouRow, label: string, tone?: ButtonTone) => ({
    label,
    tone,
    disabled: noAgent || sending,
    subject: row.kind === "checks" ? "manual checks" : row.id,
    theme,
  });

  const answerForm = (row: NeedsYouRow) => {
    const submit = async () => {
      if (draft.trim() && (await run(answerPrompt(row.id, row.title, draft)))) {
        setAnswering(null);
        setDraft("");
      }
    };
    return (
      <View style={{ gap: 6, marginTop: 6 }}>
        <TextInput
          accessibilityLabel={`Answer ${row.id}`}
          autoFocus
          value={draft}
          onChangeText={setDraft}
          onSubmitEditing={() => void submit()}
          placeholder="Your answer"
          placeholderTextColor={colors.foregroundMuted}
          style={{
            paddingHorizontal: 9,
            paddingVertical: 6,
            borderRadius: 7,
            borderWidth: 1,
            borderColor: colors.border,
            fontSize: 12.5,
            color: colors.foreground,
          }}
        />
        <View style={{ flexDirection: "row", gap: 6 }}>
          <ActionButton
            {...button(row, "Send", "primary")}
            disabled={noAgent || sending || !draft.trim()}
            onPress={() => void submit()}
          />
          <ActionButton
            {...button(row, "Cancel")}
            disabled={sending}
            onPress={() => {
              setAnswering(null);
              setDraft("");
            }}
          />
        </View>
      </View>
    );
  };

  const buttons = (row: NeedsYouRow) => {
    if (row.kind === "question") {
      const defaultAnswer = row.defaultAnswer;
      return (
        <>
          <ActionButton
            {...button(row, "Answer")}
            onPress={() => {
              setRejecting(null);
              setDraft("");
              setAnswering(row.id);
            }}
          />
          {defaultAnswer ? (
            <ActionButton
              {...button(row, "Use default")}
              onPress={() => void run(defaultAnswerPrompt(row.id, row.title, defaultAnswer))}
            />
          ) : null}
        </>
      );
    }
    if (row.kind === "adr") {
      const confirming = rejecting === row.id;
      return (
        <>
          {DECISION_ID.test(row.id) ? (
            <ActionButton
              {...button(row, "View")}
              disabled={false}
              onPress={() => onOpenDecision({ id: row.id, title: row.title })}
            />
          ) : null}
          <ActionButton
            {...button(row, "Accept")}
            onPress={() => {
              setRejecting(null);
              void run(acceptDecisionPrompt(row.id, row.title));
            }}
          />
          <ActionButton
            {...button(row, confirming ? "Confirm reject" : "Reject", "danger")}
            onPress={() => {
              if (confirming) {
                void run(rejectDecisionPrompt(row.id, row.title)).finally(() => setRejecting(null));
              } else {
                setRejecting(row.id);
              }
            }}
          />
          {confirming ? (
            <ActionButton
              {...button(row, "Cancel")}
              disabled={sending}
              onPress={() => setRejecting(null)}
            />
          ) : null}
        </>
      );
    }
    return (
      <ActionButton
        {...button(row, "Mark done")}
        onPress={() => void run(manualChecksPrompt(row.checks ?? []))}
      />
    );
  };

  return (
    <OverviewCell
      title="Needs you"
      aside={rows.length ? String(rows.length) : undefined}
      tone={rows.some((row) => row.tone === "danger") ? "warning" : undefined}
      theme={theme}
    >
      {shown.length === 0 ? <Empty text="Nothing waits on you." theme={theme} /> : null}
      {note && target?.state === "ready" && onOpenAgent ? (
        <Pressable
          accessibilityRole="link"
          accessibilityLabel={`Open ${target.name}, the thread that receives these actions`}
          onPress={() => onOpenAgent(target.id)}
          style={({ pressed }) => ({ alignSelf: "flex-start", opacity: pressed ? 0.6 : 1 })}
        >
          <Text
            numberOfLines={1}
            style={{ fontSize: 11, marginBottom: 2, color: colors.foregroundMuted }}
          >
            Actions go to{" "}
            <Text style={{ color: colors.accent, textDecorationLine: "underline" }}>
              {target.name}
            </Text>
          </Text>
        </Pressable>
      ) : note ? (
        <Text
          numberOfLines={1}
          style={{
            fontSize: 11,
            marginBottom: 2,
            color: note.warning ? colors.statusWarning : colors.foregroundMuted,
          }}
        >
          {note.text}
        </Text>
      ) : null}
      {shown.map((row, index) => {
        const icon = rowIcon(row, theme);
        const rowKey = `${row.kind}-${row.id}`;
        const open = expanded === rowKey;
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
              <Pressable
                accessibilityRole="button"
                accessibilityState={{ expanded: open }}
                accessibilityLabel={`${open ? "Collapse" : "Show all of"} ${row.id}: ${row.title}`}
                onPress={() => setExpanded(open ? null : rowKey)}
              >
                <Text
                  numberOfLines={open ? undefined : 2}
                  style={{ fontSize: 12.5, lineHeight: 17, color: colors.foreground }}
                >
                  {row.title}
                </Text>
                {open && row.kind === "question" && row.defaultAnswer ? (
                  <Text
                    style={{
                      fontSize: 11.5,
                      lineHeight: 16,
                      color: colors.foreground,
                      marginTop: 3,
                    }}
                  >
                    <Text style={{ fontWeight: "600" }}>Default: </Text>
                    {row.defaultAnswer}
                  </Text>
                ) : null}
              </Pressable>
              <Text
                numberOfLines={1}
                style={{ fontSize: 11, color: colors.foregroundMuted, marginTop: 1 }}
              >
                {row.meta}
                {open ? "" : " · press to read all"}
              </Text>
              {answering === row.id ? (
                answerForm(row)
              ) : (
                <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 6, marginTop: 6 }}>
                  {buttons(row)}
                </View>
              )}
            </View>
          </View>
        );
      })}
      <ShowMore {...more} theme={theme} />
    </OverviewCell>
  );
}
