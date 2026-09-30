import type { PluginSurfaceProps } from "@getpaseo/plugin/client";
import { useRpc } from "@getpaseo/plugin/client";
import { copyText } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { ActivityIndicator, Pressable, Text, View } from "react-native";
import { recapDayMarkdown, recapsRpc, type RecapDay, type RecapEntry } from "../shared/recaps";

const CARD_RADIUS = 12;
const CONTROL_RADIUS = 8;

export function BoardViewSwitch({
  view,
  onChange,
  theme,
}: {
  view: "runs" | "recaps";
  onChange: (view: "runs" | "recaps") => void;
  theme: PluginSurfaceProps["theme"];
}) {
  const colors = theme.colors;
  return (
    <View
      accessibilityRole="tablist"
      style={{
        flexDirection: "row",
        borderWidth: 1,
        borderColor: colors.border,
        borderRadius: 10,
        overflow: "hidden",
        backgroundColor: colors.surface1,
      }}
    >
      {(["runs", "recaps"] as const).map((item, index) => (
        <Pressable
          key={item}
          accessibilityRole="tab"
          accessibilityState={{ selected: view === item }}
          onPress={() => onChange(item)}
          style={({ pressed }) => ({
            height: 44,
            paddingHorizontal: 14,
            alignItems: "center",
            justifyContent: "center",
            borderLeftWidth: index ? 1 : 0,
            borderColor: colors.border,
            backgroundColor: view === item || pressed ? colors.surface2 : colors.surface1,
          })}
        >
          <Text
            style={{
              color: view === item ? colors.foreground : colors.foregroundMuted,
              fontSize: 13,
              fontWeight: view === item ? "600" : "500",
            }}
          >
            {item === "runs" ? "Runs" : "Recaps"}
          </Text>
        </Pressable>
      ))}
    </View>
  );
}

function RecapRow({
  entry,
  s,
  theme,
  onOpen,
}: {
  entry: RecapEntry;
  s: (value: number) => number;
  theme: PluginSurfaceProps["theme"];
  onOpen?: (agentId: string) => void;
}) {
  const colors = theme.colors;
  const meta = [entry.branch, entry.commitPush].filter(Boolean).join(" · ");
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`Open conversation ${entry.title}`}
      disabled={!onOpen}
      onPress={() => onOpen?.(entry.agentId)}
      style={({ pressed }) => ({
        gap: s(3),
        paddingVertical: s(8),
        paddingHorizontal: s(10),
        borderRadius: s(CARD_RADIUS - 2),
        borderWidth: 1,
        borderColor: colors.border,
        backgroundColor: pressed ? colors.surface2 : colors.surface1,
      })}
    >
      <Text
        numberOfLines={1}
        style={{
          color: colors.foreground,
          fontSize: s(13.5),
          lineHeight: s(19),
          fontWeight: "600",
        }}
      >
        {entry.title}
      </Text>
      {entry.did ? (
        <Text style={{ color: colors.foreground, fontSize: s(13), lineHeight: s(18) }}>
          {entry.did}
        </Text>
      ) : null}
      {meta ? (
        <Text
          numberOfLines={1}
          style={{ color: colors.foregroundMuted, fontSize: s(12), lineHeight: s(16) }}
        >
          {meta}
        </Text>
      ) : null}
    </Pressable>
  );
}

function RecapDaySection({
  day,
  s,
  theme,
  hues,
  onOpen,
}: {
  day: RecapDay;
  s: (value: number) => number;
  theme: PluginSurfaceProps["theme"];
  hues: Record<string, number>;
  onOpen?: (agentId: string) => void;
}) {
  const colors = theme.colors;
  const [copy, setCopy] = useState<"idle" | "copied" | "failed">("idle");
  const onCopy = () =>
    void copyText(recapDayMarkdown(day)).then(
      () => setCopy("copied"),
      () => setCopy("failed"),
    );
  return (
    <View style={{ gap: s(10) }}>
      <View
        style={{
          flexDirection: "row",
          alignItems: "center",
          gap: s(8),
          paddingBottom: s(8),
          borderBottomWidth: 1,
          borderBottomColor: colors.border,
        }}
      >
        <Text
          accessibilityRole="header"
          style={{ flex: 1, color: colors.foreground, fontSize: s(15), fontWeight: "600" }}
        >
          {day.day}
        </Text>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`Copy recaps for ${day.day} as markdown`}
          onPress={onCopy}
          style={({ pressed }) => ({
            paddingHorizontal: s(12),
            paddingVertical: s(6),
            borderRadius: s(CONTROL_RADIUS),
            backgroundColor: pressed ? colors.surface1 : colors.surface2,
          })}
        >
          <Text style={{ color: colors.foreground, fontSize: s(13), fontWeight: "600" }}>
            {copy === "copied" ? "Copied" : copy === "failed" ? "Copy failed" : "Copy"}
          </Text>
        </Pressable>
      </View>
      {day.projects.map((project) => {
        const hue = project.projectId === undefined ? undefined : hues[project.projectId];
        return (
          <View key={project.key} style={{ gap: s(6) }}>
            <View style={{ flexDirection: "row", alignItems: "center", gap: s(8) }}>
              <View
                accessibilityElementsHidden
                style={{
                  width: s(12),
                  height: s(12),
                  borderRadius: s(4),
                  backgroundColor:
                    hue === undefined ? colors.foregroundMuted : `hsl(${hue}, 42%, 58%)`,
                }}
              />
              <Text
                accessibilityRole="header"
                numberOfLines={1}
                style={{ color: colors.foreground, fontSize: s(13), fontWeight: "600" }}
              >
                {project.name}
              </Text>
            </View>
            {project.entries.map((entry) => (
              <RecapRow
                key={`${entry.agentId}:${entry.endedAt}`}
                entry={entry}
                s={s}
                theme={theme}
                onOpen={onOpen}
              />
            ))}
          </View>
        );
      })}
    </View>
  );
}

export function RecapsView({
  hostId,
  theme,
  scale,
  hues,
  onOpen,
}: {
  hostId: string;
  theme: PluginSurfaceProps["theme"];
  scale: number;
  hues: Record<string, number>;
  onOpen?: (agentId: string) => void;
}) {
  const readRecaps = useRpc(recapsRpc);
  const recaps = useQuery({
    queryKey: ["board-recaps", hostId],
    queryFn: () => readRecaps({ days: 7 }),
    retry: false,
    refetchInterval: 10_000,
    refetchOnWindowFocus: false,
  });
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const message = (text: string) => (
    <View
      style={{
        minHeight: s(88),
        alignItems: "center",
        justifyContent: "center",
        padding: s(16),
        borderRadius: s(CARD_RADIUS),
        borderWidth: 1,
        borderStyle: "dashed",
        borderColor: colors.border,
      }}
    >
      <Text style={{ color: colors.foregroundMuted, fontSize: s(13), textAlign: "center" }}>
        {text}
      </Text>
    </View>
  );
  if (recaps.isPending) return <ActivityIndicator color={colors.accent} />;
  if (!recaps.data) return message("Could not load recaps. Retrying.");
  if (!recaps.data.days.length) return message("No recaps in the last 7 days.");
  return (
    <View style={{ gap: s(24) }}>
      {recaps.data.days.map((day) => (
        <RecapDaySection key={day.day} day={day} s={s} theme={theme} hues={hues} onOpen={onOpen} />
      ))}
    </View>
  );
}
