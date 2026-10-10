import { type PluginHostProps, useRpc } from "@getpaseo/plugin/client";
import { Icon, Modal } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Pressable, Text, View } from "react-native";
import { readDecisionRpc } from "../shared/overview";
import { type MarkdownBlock, parseMarkdown } from "./markdown";
import { MONO } from "./overview-cells";

export interface DecisionTarget {
  id: string;
  title: string;
}

type DecisionModalProps = Pick<PluginHostProps, "host" | "theme"> & {
  workspaceId: string;
  // Null keeps the modal closed.
  target: DecisionTarget | null;
  onClose(): void;
};

function Block({ block, theme }: { block: MarkdownBlock } & Pick<PluginHostProps, "theme">) {
  const { colors } = theme;
  const text = { color: colors.foreground, fontSize: 13, lineHeight: 19 };
  switch (block.kind) {
    case "heading":
      return (
        <Text
          accessibilityRole="header"
          selectable
          style={{
            ...text,
            fontWeight: "700",
            fontSize: block.level === 1 ? 17 : block.level === 2 ? 15 : 13.5,
            lineHeight: block.level === 1 ? 24 : 21,
            marginTop: block.level === 1 ? 0 : 8,
          }}
        >
          {block.text}
        </Text>
      );
    case "bullet":
      return (
        <View style={{ flexDirection: "row", gap: 8, marginLeft: block.depth * 14 }}>
          <Text style={{ ...text, color: colors.foregroundMuted }}>•</Text>
          <Text selectable style={{ ...text, flex: 1 }}>
            {block.text}
          </Text>
        </View>
      );
    case "code":
      return (
        <View style={{ padding: 10, borderRadius: 8, backgroundColor: colors.surface2 }}>
          <Text selectable style={{ ...text, fontFamily: MONO, fontSize: 11.5, lineHeight: 17 }}>
            {block.text}
          </Text>
        </View>
      );
    default:
      return (
        <Text selectable style={text}>
          {block.text}
        </Text>
      );
  }
}

function DecisionBody({
  target,
  workspaceId,
  host,
  theme,
}: Pick<DecisionModalProps, "workspaceId" | "host" | "theme"> & { target: DecisionTarget }) {
  const { colors } = theme;
  const read = useRpc(readDecisionRpc);
  const decision = useQuery({
    queryKey: ["watchtower-decision", host.id, workspaceId, target.id],
    queryFn: () => read({ workspaceId, id: target.id }),
    retry: false,
  });
  const markdown = decision.data?.markdown;
  const blocks = useMemo(() => (markdown === undefined ? [] : parseMarkdown(markdown)), [markdown]);
  if (decision.error) {
    return (
      <View accessibilityRole="alert" style={{ gap: 10, alignItems: "flex-start" }}>
        <Text style={{ fontSize: 13, color: colors.statusDanger }}>
          Could not load {target.id}. {decision.error.message}
        </Text>
        <Pressable
          accessibilityRole="button"
          onPress={() => void decision.refetch()}
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
  }
  if (!decision.data) {
    return (
      <Text accessibilityLabel={`Loading ${target.id}`} style={{ color: colors.foregroundMuted }}>
        Loading…
      </Text>
    );
  }
  return (
    <>
      <Text selectable style={{ fontFamily: MONO, fontSize: 11, color: colors.foregroundMuted }}>
        {decision.data.file}
      </Text>
      {blocks.map((block, index) => (
        <Block key={index} block={block} theme={theme} />
      ))}
    </>
  );
}

// The query runs only while the modal is open, because the body mounts with `target`.
export function DecisionModal({ target, workspaceId, host, theme, onClose }: DecisionModalProps) {
  return (
    <Modal
      title={target ? `${target.id} ${target.title}` : "Decision"}
      icon={<Icon name="Scale" size={18} color={theme.colors.foreground} />}
      open={target !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <Modal.Content contentContainerStyle={{ padding: 20, gap: 8 }}>
        {target ? (
          <DecisionBody target={target} workspaceId={workspaceId} host={host} theme={theme} />
        ) : null}
      </Modal.Content>
    </Modal>
  );
}
