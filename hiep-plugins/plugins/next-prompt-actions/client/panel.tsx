import type { PluginTheme } from "@getpaseo/plugin";
import type { PluginClientContext, PluginTimelineItemProps } from "@getpaseo/plugin/client";
import { Button as HostButton, Markdown, usePrimaryModifier } from "@getpaseo/plugin/client/ui";
import { Icon } from "@getpaseo/plugin/client/react-native";
import { useId, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Platform, Pressable, Text, View } from "react-native";
import { z } from "zod";
import {
  inspectRpc,
  scopeRpc,
  sendRpc,
  startRpc,
  type Candidate,
  type Scope,
} from "../shared/contracts";
import { selectionAllowed } from "../shared/next-prompts";
import { gitAction, joinPrompts, parsePrompts, type PromptBlock } from "../shared/prompts";
import { commitChip, recapSections } from "../shared/section";
import { layout, reachable, type Group } from "../shared/selection";
import { backToBoard } from "./back-to-board";

export const panelSchema = z.object({
  title: z.string(),
  intro: z.string(),
  section: z.string(),
  recap: z
    .object({
      branch: z.string(),
      did: z.string(),
      commit: z.string(),
      // Five-field Recap only. Optional, so rows stored before these fields stay valid.
      notYet: z.string().optional(),
      need: z.string().optional(),
    })
    .optional(),
});
type PanelData = z.output<typeof panelSchema>;
type Client = Pick<PluginClientContext, "rpc" | "setComposerText" | "openSurface">;
interface Actions {
  busy: boolean;
  pending: boolean;
  modifier: boolean;
  compact: boolean;
  touch: boolean;
  edit(texts: string[]): void;
  send(keys: string[]): void;
  start(key: string): void;
}

function Button({
  label,
  onPress,
  disabled,
  primary,
  theme,
  icon,
  faces,
  onHoverIn,
  onHoverOut,
  quiet = false,
}: {
  label: string;
  onPress(): void;
  disabled?: boolean;
  primary?: boolean;
  theme: PluginTheme;
  icon?: string;
  faces?: { label: string; icon: string }[];
  onHoverIn?(): void;
  onHoverOut?(): void;
  quiet?: boolean;
}) {
  const ink = primary
    ? theme.colors.surface0
    : quiet
      ? theme.colors.foregroundMuted
      : theme.colors.foreground;
  const face = (text: string, name?: string) => (
    <View
      style={{
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "center",
        gap: 6,
        flexShrink: 0,
      }}
    >
      {name ? <Icon name={name} size={14} color={ink} /> : null}
      <Text
        numberOfLines={1}
        style={{ color: ink, fontSize: 13, lineHeight: 16, fontWeight: "500", flexShrink: 0 }}
      >
        {text}
      </Text>
    </View>
  );
  return (
    <HostButton
      size="sm"
      variant="outline"
      accessibilityLabel={label}
      onPress={onPress}
      onHoverIn={onHoverIn}
      onHoverOut={onHoverOut}
      disabled={disabled}
      style={{
        flexShrink: 0,
        minHeight: Platform.OS === "web" ? 32 : 44,
        minWidth: faces || primary ? 80 : undefined,
        paddingVertical: 0,
        borderRadius: 8,
        paddingHorizontal: quiet ? 8 : 12,
        borderColor: quiet
          ? "transparent"
          : primary
            ? theme.colors.foreground
            : theme.colors.border,
        backgroundColor: quiet
          ? "transparent"
          : primary
            ? theme.colors.foreground
            : theme.colors.surface0,
      }}
      trailing={
        faces ? (
          <View style={{ height: 16, justifyContent: "center", alignItems: "center" }}>
            {/* Like the original grid faces: both labels reserve width, only one is painted. */}
            <View
              aria-hidden
              accessible={false}
              style={{ opacity: 0, height: 16, overflow: "hidden", alignItems: "center" }}
            >
              {faces.map((item) => (
                <View key={item.label}>{face(item.label, item.icon)}</View>
              ))}
            </View>
            <View
              style={{
                position: "absolute",
                inset: 0,
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {face(label, icon)}
            </View>
          </View>
        ) : (
          face(label, icon)
        )
      }
    />
  );
}

// Display-only mark for a prompt the agent recommends. Plain text, so assistive tech reads it.
// The fixed fill keeps white text above AA contrast in both themes.
function Suggested({ id }: { id?: string }) {
  return (
    <View
      style={{
        alignSelf: "flex-end",
        minHeight: 20,
        justifyContent: "center",
        paddingHorizontal: 8,
        borderRadius: 6,
        backgroundColor: "#0f7b5f",
      }}
    >
      <Text
        nativeID={id}
        style={{ color: "#fff", fontSize: 11.5, lineHeight: 15, fontWeight: "600" }}
      >
        Suggested
      </Text>
    </View>
  );
}

function PromptCard({
  text,
  why,
  candidate,
  actions,
  theme,
  bare = false,
  suggested = false,
}: {
  text: string;
  why: string;
  candidate: Candidate | undefined;
  actions: Actions;
  theme: PluginTheme;
  bare?: boolean;
  suggested?: boolean;
}) {
  const [hovered, setHovered] = useState(false);
  const git = candidate && !candidate.thread ? gitAction(candidate.text) : null;
  const sent = candidate?.state === "sent";
  const swapped = hovered && actions.modifier && candidate?.state === "ready" && !actions.busy;
  const newThread = !git && (candidate?.thread ? !swapped : swapped);
  const ready = candidate?.state === "ready" && !actions.pending && (newThread || !actions.busy);
  const label =
    candidate?.state === "unknown"
      ? newThread
        ? "Check threads"
        : "Check chat"
      : candidate?.state === "sending"
        ? newThread
          ? "Starting..."
          : "Sending..."
        : sent
          ? candidate.thread
            ? "Started"
            : "Sent"
          : newThread
            ? candidate?.thread
              ? "Start in new thread"
              : "New thread"
            : (git?.label ?? "Send");
  const content = (
    <View
      style={{
        minWidth: 0,
        rowGap: 14,
        columnGap: 24,
        flexDirection: actions.compact || bare ? "column" : "row",
        alignItems: "flex-end",
      }}
    >
      <View
        style={{
          flex: actions.compact || bare ? undefined : 1,
          alignSelf: "stretch",
          minWidth: 0,
          gap: 4,
        }}
      >
        <Text
          selectable
          style={{ color: theme.colors.foreground, fontSize: 15, lineHeight: 24, flexShrink: 1 }}
        >
          {text}
        </Text>
        {why ? (
          <Markdown
            text={why}
            compact
            enableHtmlish={false}
            textStyle={{ color: theme.colors.foregroundMuted, fontSize: 13, lineHeight: 20 }}
          />
        ) : null}
      </View>
      {candidate ? (
        <View
          style={{
            flexDirection: "row",
            flexShrink: 0,
            alignItems: "center",
            alignSelf: "flex-end",
            justifyContent: "flex-end",
            gap: 8,
            flexWrap: "wrap",
          }}
        >
          {!candidate.thread ? (
            <Button
              label="Edit"
              icon="Pencil"
              theme={theme}
              disabled={actions.busy}
              onPress={() => actions.edit([text])}
            />
          ) : null}
          <Button
            primary={!newThread}
            faces={
              !git
                ? [
                    {
                      label: candidate.thread ? "Start in new thread" : "Send",
                      icon: candidate.thread ? "SquarePen" : "ArrowUp",
                    },
                    {
                      label: candidate.thread ? "Send" : "New thread",
                      icon: candidate.thread ? "ArrowUp" : "SquarePen",
                    },
                  ]
                : undefined
            }
            onHoverIn={() => setHovered(true)}
            onHoverOut={() => setHovered(false)}
            label={label}
            icon={
              sent ? "Check" : newThread ? "SquarePen" : git ? "GitCommitHorizontal" : "ArrowUp"
            }
            theme={theme}
            disabled={!ready}
            onPress={() =>
              newThread ? actions.start(candidate.key) : actions.send([candidate.key])
            }
          />
          {actions.touch && !candidate.thread && !git ? (
            <Button
              label="New thread"
              icon="SquarePen"
              theme={theme}
              disabled={!ready}
              onPress={() => actions.start(candidate.key)}
            />
          ) : null}
        </View>
      ) : null}
    </View>
  );
  if (bare) return content;
  // The badge has its own row at the card's top right, so it never covers text or buttons.
  return (
    <View
      style={{
        minWidth: 0,
        gap: 8,
        padding: 16,
        paddingLeft: 18,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 8,
        backgroundColor: theme.colors.surface0,
      }}
    >
      {suggested ? <Suggested /> : null}
      {content}
    </View>
  );
}

// The reply's own fence decides, so earlier replies without server candidates keep their badge.
const isSuggested = (block: PromptBlock, index: number, candidate: Candidate | undefined) =>
  block.suggestions?.[index] === true || candidate?.suggestion === true;

function groupTitle(group: Group): string {
  switch (group.kind) {
    case "choice":
      return "Choose one";
    case "together":
      return group.set ? `Send together · Set ${group.set}` : "Send together";
    case "follow-up":
      return "Follow-up";
    case "alone":
      return "Send alone";
  }
}

function SelectionBlock({
  block,
  candidates,
  actions,
  theme,
}: {
  block: PromptBlock;
  candidates: (Candidate | undefined)[];
  actions: Actions;
  theme: PluginTheme;
}) {
  const declaration = block.declaration!;
  const [selected, setSelected] = useState<string[]>([]);
  const badgeId = useId();
  const ids = declaration.prompts.filter((prompt) => !prompt.thread).map((prompt) => prompt.id);
  const groups = layout(ids, declaration.exclusiveGroups, declaration.allowedCombinations);
  const indexOf = (id: string) => declaration.prompts.findIndex((prompt) => prompt.id === id);
  const ordered = ids.filter((id) => selected.includes(id));
  const chosen = ordered.map((id) => candidates[indexOf(id)]);
  const canSend =
    chosen.length > 0 &&
    chosen.every((candidate) => candidate?.state === "ready") &&
    selectionAllowed(chosen as Candidate[]) &&
    !actions.busy;
  const toggle = (id: string) => {
    if (selected.includes(id)) return setSelected(selected.filter((value) => value !== id));
    const group = declaration.exclusiveGroups.find((ids) => ids.includes(id)) ?? [];
    setSelected([...selected.filter((value) => !group.includes(value)), id]);
  };
  return (
    <View style={{ gap: 16, minWidth: 0 }}>
      {groups.map((group) => (
        <View key={`${group.kind}:${group.ids.join(",")}`} style={{ gap: 8 }}>
          <Text style={{ color: theme.colors.foreground, fontSize: 13, fontWeight: "600" }}>
            {groupTitle(group)}
          </Text>
          <View
            style={{
              borderWidth: 1,
              borderColor: theme.colors.border,
              borderRadius: 8,
              overflow: "hidden",
            }}
          >
            {group.ids.map((id, row) => {
              const index = indexOf(id);
              const on = selected.includes(id);
              const allowed =
                on ||
                reachable(
                  selected,
                  id,
                  declaration.exclusiveGroups,
                  declaration.allowedCombinations,
                );
              const candidate = candidates[index];
              const disabled = !allowed || actions.busy || candidate?.state !== "ready";
              const status =
                candidate?.state === "sent"
                  ? "Sent"
                  : candidate?.state === "unknown"
                    ? "Check chat"
                    : candidate?.state === "sending"
                      ? "Sending..."
                      : allowed
                        ? ""
                        : declaration.allowedCombinations.some((combo) => combo.includes(id))
                          ? "Not with selection"
                          : "Send alone";
              const radio = group.kind === "choice";
              const suggested = isSuggested(block, index, candidate);
              // The label names the prompt; this adds the badge to what a screen reader announces.
              const described = !suggested
                ? {}
                : Platform.OS === "web"
                  ? { "aria-describedby": `${badgeId}-${id}` }
                  : { accessibilityHint: "Suggested" };
              return (
                <Pressable
                  key={id}
                  accessibilityRole={radio ? "radio" : "checkbox"}
                  accessibilityLabel={block.prompts[index]}
                  accessibilityState={{ checked: on, disabled }}
                  {...described}
                  disabled={disabled}
                  onPress={() => toggle(id)}
                  style={{
                    gap: 8,
                    padding: 14,
                    minWidth: 0,
                    borderTopWidth: row ? 1 : 0,
                    borderTopColor: theme.colors.border,
                    backgroundColor: on ? theme.colors.surface1 : theme.colors.surface0,
                    opacity: allowed ? 1 : 0.5,
                  }}
                >
                  {suggested ? <Suggested id={`${badgeId}-${id}`} /> : null}
                  <View style={{ flexDirection: "row", gap: 12, minWidth: 0 }}>
                    <View
                      style={{
                        height: 18,
                        width: 18,
                        marginTop: 3,
                        borderRadius: radio ? 9 : 4,
                        borderWidth: 1,
                        borderColor: on ? theme.colors.accent : theme.colors.border,
                        alignItems: "center",
                        justifyContent: "center",
                      }}
                    >
                      {on ? (
                        <Icon
                          name={radio ? "Circle" : "Check"}
                          size={12}
                          color={theme.colors.accent}
                        />
                      ) : null}
                    </View>
                    <View style={{ flex: 1, minWidth: 0 }}>
                      <PromptCard
                        bare
                        text={block.prompts[index]}
                        why={block.whys[index]}
                        candidate={undefined}
                        actions={actions}
                        theme={theme}
                      />
                    </View>
                    {status ? (
                      <Text style={{ color: theme.colors.foregroundMuted, fontSize: 12 }}>
                        {status}
                      </Text>
                    ) : null}
                  </View>
                </Pressable>
              );
            })}
          </View>
        </View>
      ))}
      {ids.length ? (
        <View
          style={{
            flexDirection: "row",
            columnGap: 16,
            rowGap: 12,
            alignItems: "center",
            flexWrap: "wrap",
            justifyContent: "space-between",
            paddingVertical: 12,
            paddingHorizontal: actions.compact ? 16 : 20,
            marginHorizontal: actions.compact ? -16 : -20,
            borderTopWidth: 1,
            borderTopColor: theme.colors.border,
            backgroundColor: theme.colors.surface1,
          }}
        >
          <View style={{ minWidth: 140 }}>
            <Text style={{ color: theme.colors.foreground, fontSize: 13, fontWeight: "600" }}>
              {selected.length} selected
            </Text>
            <Text style={{ color: theme.colors.foregroundMuted, fontSize: 12.5 }}>
              {ordered.length > 1
                ? selectionAllowed(chosen as Candidate[])
                  ? "One numbered message"
                  : "Add or remove a suggestion to match a set"
                : ordered.length
                  ? "One prompt"
                  : "Choose a suggestion to continue"}
            </Text>
          </View>
          <View
            style={{
              flexDirection: "row",
              alignItems: "center",
              justifyContent: "flex-end",
              flexWrap: "wrap",
              gap: 8,
            }}
          >
            <Button
              quiet
              label="Clear"
              theme={theme}
              disabled={actions.pending || !selected.length}
              onPress={() => setSelected([])}
            />
            <Button
              label="Edit selected"
              icon="Pencil"
              theme={theme}
              disabled={!canSend}
              onPress={() => actions.edit(ordered.map((id) => block.prompts[indexOf(id)]))}
            />
            <Button
              primary
              label={
                actions.pending
                  ? "Sending..."
                  : chosen.some((candidate) => candidate?.state === "unknown")
                    ? "Check chat"
                    : chosen.length && chosen.every((candidate) => candidate?.state === "sent")
                      ? "Sent"
                      : chosen.some((candidate) => candidate?.state === "sending")
                        ? "Sending..."
                        : ordered.length
                          ? `Send selected (${ordered.length})`
                          : "Send selected"
              }
              icon="ArrowUp"
              theme={theme}
              disabled={!canSend}
              onPress={() => actions.send(chosen.map((candidate) => candidate!.key))}
            />
          </View>
        </View>
      ) : null}
    </View>
  );
}

/** Renders the What Next section of a reply. Only the latest reply's suggestions have buttons. */
export function createNextPromptPanel(client: Client) {
  return function NextPromptPanel({
    agentId,
    item,
    theme,
    host,
    timestamp,
  }: PluginTimelineItemProps<PanelData>) {
    const modifier = usePrimaryModifier();
    const [width, setWidth] = useState(0);
    const [pending, setPending] = useState(false);
    const [error, setError] = useState("");
    const blocks = useMemo(() => parsePrompts(item.data.section), [item.data.section]);
    // Shared by this agent's mounted replies; refresh external sends and Jev changes too.
    const inspection = useQuery({
      queryKey: ["next-prompt-inspection", host.id, agentId],
      queryFn: async () => {
        const scope = await client.rpc(scopeRpc, { agentId });
        return { scope, snapshot: await client.rpc(inspectRpc, scope) };
      },
      refetchInterval: 2500,
      retry: false,
    });
    const { scope, snapshot } = inspection.data ?? {};
    const run = (task: (target: Scope) => Promise<unknown>) => {
      if (!scope) return;
      setPending(true);
      setError("");
      void task(scope)
        .catch((failure: unknown) => setError(String(failure)))
        .finally(() => {
          setPending(false);
          void inspection.refetch();
        });
    };
    const actions: Actions = {
      pending,
      modifier,
      compact: width <= 600,
      touch: Platform.OS !== "web",
      busy:
        pending || inspection.isError || !!snapshot?.busy || snapshot?.note === "Jev reviewing...",
      edit: (texts) => client.setComposerText({ agentId, text: joinPrompts(texts) }),
      send: (keys) =>
        run(async (target) => {
          // Leave at once; the Board refreshes when the prompt lands.
          if (backToBoard.enabled) client.openSurface("board", { pluginId: "board" });
          await client.rpc(sendRpc, { ...target, key: keys.length === 1 ? keys[0] : keys });
        }),
      start: (key) => run((target) => client.rpc(startRpc, { ...target, key })),
    };
    const candidateFor = (block: PromptBlock, text: string) =>
      snapshot?.candidates.find(
        (candidate) =>
          // Streamed and stored reply timestamps differ; use the original user-turn boundary.
          candidate.after !== undefined &&
          timestamp.getTime() > candidate.after &&
          candidate.block === block.block &&
          candidate.text === text,
      );
    const recap = item.data.recap;
    const sections = recap ? recapSections(recap) : null;
    const commit = recap ? commitChip(recap.commit) : null;
    const done = blocks.some((block) => block.declaration?.goal === "done");
    return (
      <View
        onLayout={(event) => setWidth(event.nativeEvent.layout.width)}
        style={{
          minWidth: 0,
          width: "100%",
          marginVertical: 6,
          borderRadius: 12,
          borderWidth: 1,
          borderColor: theme.colors.border,
          backgroundColor: theme.colors.surface0,
          overflow: "hidden",
        }}
      >
        {recap ? (
          <View
            style={{
              paddingHorizontal: 20,
              paddingVertical: 14,
              gap: 6,
              borderBottomWidth: 1,
              borderBottomColor: theme.colors.border,
              backgroundColor: theme.colors.surface1,
            }}
          >
            <View style={{ flexDirection: "row", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
              <Icon name="History" size={16} color={theme.colors.foregroundMuted} />
              <Text
                style={{
                  color: theme.colors.foreground,
                  fontSize: 15,
                  fontWeight: "600",
                  marginRight: "auto",
                }}
              >
                Recap
              </Text>
              {[
                { text: recap.branch.replace(/^`|`$/g, "") },
                // State reads from the icon and wording, not an extra hue.
                { ...commit!, icon: commit!.done ? "Check" : "GitCommitHorizontal" },
                ...(done ? [{ text: "Task done" }] : []),
              ].map((chip: { text: string; icon?: string; label?: string }, index) => (
                <View
                  key={index}
                  style={{
                    flexDirection: "row",
                    alignItems: "center",
                    gap: 5,
                    backgroundColor: theme.colors.surface2,
                    paddingHorizontal: 8,
                    paddingVertical: 3,
                    borderRadius: 6,
                    maxWidth: "100%",
                  }}
                >
                  {chip.icon ? (
                    <Icon name={chip.icon} size={13} color={theme.colors.foregroundMuted} />
                  ) : null}
                  <Text
                    accessibilityLabel={chip.label}
                    style={{ color: theme.colors.foregroundMuted, fontSize: 12, flexShrink: 1 }}
                  >
                    {chip.text}
                  </Text>
                </View>
              ))}
            </View>
            {/* Five fields: each section has its label above its value, divided from the one before. */}
            {(sections ?? [{ label: "", text: recap.did }]).map((section, index) => {
              const value = (
                <Markdown
                  key={section.label}
                  text={section.text}
                  compact
                  enableHtmlish={false}
                  textStyle={{ color: theme.colors.foregroundMuted, fontSize: 14, lineHeight: 22 }}
                />
              );
              return sections ? (
                <View
                  key={section.label}
                  style={
                    index
                      ? {
                          marginTop: 4,
                          paddingTop: 10,
                          borderTopWidth: 1,
                          borderTopColor: theme.colors.border,
                        }
                      : { marginTop: 2 }
                  }
                >
                  <Text
                    style={{
                      color: theme.colors.foregroundMuted,
                      fontSize: 12.5,
                      lineHeight: 20,
                      fontWeight: "600",
                      marginBottom: 2,
                    }}
                  >
                    {section.label}
                  </Text>
                  {value}
                </View>
              ) : (
                value
              );
            })}
          </View>
        ) : null}
        <View style={{ padding: width <= 600 ? 16 : 20, gap: 16, minWidth: 0 }}>
          <View style={{ gap: 4 }}>
            <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
              <Icon name="CornerDownRight" size={16} color={theme.colors.accent} />
              <Text style={{ color: theme.colors.foreground, fontSize: 15, fontWeight: "600" }}>
                {item.data.title}
              </Text>
              {done && !recap ? (
                <Text style={{ fontSize: 12, color: theme.colors.foregroundMuted }}>Task done</Text>
              ) : null}
            </View>
            {item.data.intro ? (
              <Markdown text={item.data.intro} compact enableHtmlish={false} />
            ) : null}
          </View>
          {blocks.map((block) => {
            const local = block.prompts
              .map((text, index) => ({ text, index }))
              .filter(({ index }) => !block.threads[index]);
            const candidates = block.prompts.map((text) => candidateFor(block, text));
            return block.declaration &&
              local.length > 1 &&
              local.every(({ index }) => candidates[index]) &&
              !local.some(({ text }) => gitAction(text)) ? (
              <SelectionBlock
                key={block.block}
                block={block}
                candidates={candidates}
                actions={actions}
                theme={theme}
              />
            ) : (
              local.map(({ text, index }) => (
                <PromptCard
                  key={`${block.block}:${index}`}
                  text={text}
                  why={block.whys[index]}
                  candidate={candidates[index]}
                  suggested={isSuggested(block, index, candidates[index])}
                  actions={actions}
                  theme={theme}
                />
              ))
            );
          })}
          {blocks.some((block) => block.threads.some(Boolean)) ? (
            <View accessibilityRole="summary" accessibilityLabel="Other work" style={{ gap: 8 }}>
              <Text
                style={{ color: theme.colors.foregroundMuted, fontSize: 13, fontWeight: "600" }}
              >
                Other work
              </Text>
              <Text style={{ color: theme.colors.foregroundMuted, fontSize: 12 }}>
                Not part of this task. Starts a separate thread.
              </Text>
              {blocks.flatMap((block) =>
                block.prompts.map((text, index) =>
                  block.threads[index] ? (
                    <PromptCard
                      key={`${block.block}:${index}`}
                      text={text}
                      why={block.whys[index]}
                      candidate={candidateFor(block, text)}
                      suggested={isSuggested(block, index, candidateFor(block, text))}
                      actions={actions}
                      theme={theme}
                    />
                  ) : null,
                ),
              )}
            </View>
          ) : null}
          {[error, inspection.error?.message, snapshot?.note, snapshot?.warning]
            .filter(Boolean)
            .map((notice, index) => (
              <Text key={index} style={{ color: theme.colors.foregroundMuted, fontSize: 13 }}>
                {notice}
              </Text>
            ))}
        </View>
      </View>
    );
  };
}
