import type { PluginTheme } from "@getpaseo/plugin";
import type { PluginClientContext, PluginTimelineItemProps } from "@getpaseo/plugin/client";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { z } from "zod";
import {
  inspectRpc,
  scopeRpc,
  sendRpc,
  startRpc,
  type Candidate,
  type Scope,
  type Snapshot,
} from "../shared/contracts";
import { selectionAllowed } from "../shared/next-prompts";
import { gitAction, joinPrompts, parsePrompts, type PromptBlock } from "../shared/prompts";
import { layout, reachable } from "../shared/selection";
import { backToBoard } from "./back-to-board";

export const panelSchema = z.object({ title: z.string(), intro: z.string(), section: z.string() });
type PanelData = z.output<typeof panelSchema>;
type Client = Pick<PluginClientContext, "rpc" | "setComposerText" | "openSurface">;

interface Actions {
  busy: boolean;
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
}: {
  label: string;
  onPress(): void;
  disabled?: boolean;
  primary?: boolean;
  theme: PluginTheme;
}) {
  const { colors } = theme;
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled}
      onPress={onPress}
      style={{
        paddingHorizontal: 10,
        paddingVertical: 5,
        borderRadius: 6,
        borderWidth: 1,
        borderColor: primary ? colors.accent : colors.border,
        backgroundColor: primary ? colors.accent : "transparent",
        opacity: disabled ? 0.5 : 1,
      }}
    >
      <Text style={{ fontSize: 12, color: primary ? colors.accentForeground : colors.foreground }}>
        {label}
      </Text>
    </Pressable>
  );
}

function PromptCard({
  text,
  why,
  candidate,
  actions,
  theme,
}: {
  text: string;
  why: string;
  candidate: Candidate | undefined;
  actions: Actions;
  theme: PluginTheme;
}) {
  const ready = candidate?.state === "ready" && !actions.busy;
  const git = candidate && !candidate.thread ? gitAction(candidate.text) : null;
  const sent = candidate?.state === "sent" || candidate?.state === "sending";
  return (
    <View style={{ gap: 6, paddingVertical: 6 }}>
      <Text selectable style={{ color: theme.colors.foreground, fontSize: 13, lineHeight: 19 }}>
        {text}
      </Text>
      {why ? (
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: 12 }}>{why}</Text>
      ) : null}
      {candidate ? (
        <View style={{ flexDirection: "row", gap: 6, flexWrap: "wrap" }}>
          <Button label="Edit" theme={theme} onPress={() => actions.edit([text])} />
          {candidate.thread ? (
            <Button
              primary
              label="Start in new thread"
              theme={theme}
              disabled={!ready}
              onPress={() => actions.start(candidate.key)}
            />
          ) : (
            <Button
              primary
              label={sent ? "Sent" : (git?.label ?? "Send")}
              theme={theme}
              disabled={!ready}
              onPress={() => actions.send([candidate.key])}
            />
          )}
          {!candidate.thread && !git ? (
            <Button
              label="New thread"
              theme={theme}
              disabled={!ready}
              onPress={() => actions.start(candidate.key)}
            />
          ) : null}
        </View>
      ) : null}
    </View>
  );
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
  const ids = declaration.prompts.map((prompt) => prompt.id);
  const [selected, setSelected] = useState<string[]>([]);
  const groups = layout(ids, declaration.exclusiveGroups, declaration.allowedCombinations);
  const chosen = selected.map((id) => candidates[ids.indexOf(id)]);
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
    <View style={{ gap: 4 }}>
      {groups.map((group) => (
        <View key={`${group.kind}:${group.ids.join(",")}`} style={{ gap: 2 }}>
          {group.ids.map((id) => {
            const index = ids.indexOf(id);
            const on = selected.includes(id);
            const allowed =
              on ||
              reachable(selected, id, declaration.exclusiveGroups, declaration.allowedCombinations);
            const radio = group.kind === "choice";
            return (
              <Pressable
                key={id}
                accessibilityRole={radio ? "radio" : "checkbox"}
                accessibilityState={{ checked: on, disabled: !allowed }}
                disabled={!allowed || declaration.prompts[index].thread === "new"}
                onPress={() => toggle(id)}
                style={{ flexDirection: "row", gap: 8, opacity: allowed ? 1 : 0.5 }}
              >
                <Text style={{ color: theme.colors.foregroundMuted, fontSize: 13 }}>
                  {radio ? (on ? "◉" : "○") : on ? "☑" : "☐"}
                </Text>
                <View style={{ flex: 1 }}>
                  <PromptCard
                    text={block.prompts[index]}
                    why={block.whys[index]}
                    candidate={declaration.prompts[index].thread ? candidates[index] : undefined}
                    actions={actions}
                    theme={theme}
                  />
                </View>
              </Pressable>
            );
          })}
        </View>
      ))}
      <View style={{ flexDirection: "row", gap: 6, alignItems: "center" }}>
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: 12 }}>
          {selected.length} selected
        </Text>
        <Button
          label="Edit selected"
          theme={theme}
          disabled={!selected.length}
          onPress={() => actions.edit(selected.map((id) => block.prompts[ids.indexOf(id)]))}
        />
        <Button
          primary
          label="Send selected"
          theme={theme}
          disabled={!canSend}
          onPress={() => actions.send(chosen.map((candidate) => candidate!.key))}
        />
      </View>
    </View>
  );
}

/** Renders the What Next section of a reply. Only the latest reply's suggestions have buttons. */
export function createNextPromptPanel(client: Client) {
  return function NextPromptPanel({ agentId, item, theme }: PluginTimelineItemProps<PanelData>) {
    const [scope, setScope] = useState<Scope | null>(null);
    const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
    const [pending, setPending] = useState(false);
    const [error, setError] = useState("");
    const blocks = useMemo(() => parsePrompts(item.data.section), [item.data.section]);
    const refresh = useCallback(async (target: Scope) => {
      setSnapshot(await client.rpc(inspectRpc, target));
    }, []);
    useEffect(() => {
      let active = true;
      client
        .rpc(scopeRpc, { agentId })
        .then(async (target) => {
          if (!active) return;
          setScope(target);
          await refresh(target);
        })
        .catch((failure: unknown) => active && setError(String(failure)));
      return () => {
        active = false;
      };
    }, [agentId, refresh]);
    const run = (task: (target: Scope) => Promise<unknown>) => {
      if (!scope) return;
      setPending(true);
      setError("");
      void task(scope)
        .catch((failure: unknown) => setError(String(failure)))
        .finally(() => {
          setPending(false);
          void refresh(scope).catch(() => undefined);
        });
    };
    const actions: Actions = {
      busy: pending || !!snapshot?.busy,
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
        (candidate) => candidate.block === block.block && candidate.text === text,
      );
    return (
      <View
        style={{
          gap: 8,
          marginVertical: 6,
          padding: 12,
          borderRadius: 12,
          borderWidth: 1,
          borderColor: theme.colors.border,
          backgroundColor: theme.colors.surface1,
        }}
      >
        <Text style={{ color: theme.colors.foreground, fontSize: 14, fontWeight: "600" }}>
          {item.data.title}
        </Text>
        {item.data.intro ? (
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: 13 }}>
            {item.data.intro}
          </Text>
        ) : null}
        {blocks.map((block) =>
          block.declaration && block.prompts.length > 1 ? (
            <SelectionBlock
              key={block.block}
              block={block}
              candidates={block.prompts.map((text) => candidateFor(block, text))}
              actions={actions}
              theme={theme}
            />
          ) : (
            block.prompts.map((text, index) => (
              <PromptCard
                key={`${block.block}:${index}`}
                text={text}
                why={block.whys[index]}
                candidate={candidateFor(block, text)}
                actions={actions}
                theme={theme}
              />
            ))
          ),
        )}
        {snapshot?.note || snapshot?.warning || error ? (
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: 12 }}>
            {error || snapshot?.warning || snapshot?.note}
          </Text>
        ) : null}
      </View>
    );
  };
}
