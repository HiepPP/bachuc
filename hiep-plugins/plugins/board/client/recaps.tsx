import type { PluginSurfaceProps } from "@getpaseo/plugin/client";
import type { BoardHostClient } from "./hosts";
import { copyText } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { ActivityIndicator, Pressable, Text, View } from "react-native";
import { recapDayMarkdown, recapsRpc, type RecapDay, type RecapEntry } from "../shared/recaps";
import { focusRing, keyboardFocus } from "./focus";

// Same shape lock as the Runs view: cards 8, controls 6.
const CARD_RADIUS = 8;
const CONTROL_RADIUS = 6;

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
  const [focused, setFocused] = useState<"runs" | "recaps" | null>(null);
  return (
    // Quiet segmented control: the track is the only chrome, the selected segment lifts off it.
    <View
      accessibilityRole="tablist"
      style={{
        flexDirection: "row",
        padding: 2,
        gap: 2,
        borderRadius: CARD_RADIUS,
        backgroundColor: colors.surface2,
      }}
    >
      {(["runs", "recaps"] as const).map((item) => (
        <Pressable
          key={item}
          accessibilityRole="tab"
          accessibilityState={{ selected: view === item }}
          onPress={() => onChange(item)}
          onFocus={(event) => setFocused(keyboardFocus(event) ? item : null)}
          onBlur={() => setFocused(null)}
          hitSlop={{ top: 8, bottom: 8 }}
          style={({ pressed }) => ({
            height: 28,
            paddingHorizontal: 12,
            alignItems: "center",
            justifyContent: "center",
            borderRadius: CONTROL_RADIUS,
            borderWidth: 1,
            borderColor: view === item ? colors.border : "transparent",
            backgroundColor: view === item ? colors.surface0 : "transparent",
            opacity: pressed ? 0.7 : 1,
            ...(focused === item ? focusRing(colors.accent) : null),
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
        borderRadius: s(CARD_RADIUS),
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
        <Text style={{ color: colors.foregroundMuted, fontSize: s(12.5), lineHeight: s(18) }}>
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
          paddingBottom: s(6),
          borderBottomWidth: 1,
          borderBottomColor: colors.border,
        }}
      >
        <Text
          accessibilityRole="header"
          style={{ flex: 1, color: colors.foreground, fontSize: s(13), fontWeight: "600" }}
        >
          {day.day}
        </Text>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`Copy recaps for ${day.day} as markdown`}
          onPress={onCopy}
          style={({ pressed }) => ({
            paddingHorizontal: s(10),
            paddingVertical: s(4),
            borderRadius: s(CONTROL_RADIUS),
            backgroundColor: pressed ? colors.surface2 : "transparent",
          })}
        >
          <Text style={{ color: colors.foregroundMuted, fontSize: s(12), fontWeight: "600" }}>
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
                  width: s(10),
                  height: s(10),
                  borderRadius: s(3),
                  backgroundColor:
                    hue === undefined ? colors.foregroundMuted : `hsl(${hue}, 42%, 58%)`,
                }}
              />
              <Text
                accessibilityRole="header"
                numberOfLines={1}
                style={{ color: colors.foregroundMuted, fontSize: s(12), fontWeight: "600" }}
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
  rpc,
  online,
  hostId,
  theme,
  scale,
  hues,
  onOpen,
}: {
  rpc: BoardHostClient["rpc"];
  online: boolean;
  hostId: string;
  theme: PluginSurfaceProps["theme"];
  scale: number;
  hues: Record<string, number>;
  onOpen?: (agentId: string) => void;
}) {
  const readRecaps = (input: { days: number }) => rpc(recapsRpc, input);
  const recaps = useQuery({
    queryKey: ["board-recaps", hostId],
    queryFn: () => readRecaps({ days: 7 }),
    enabled: online,
    retry: false,
    refetchInterval: 10_000,
    refetchOnWindowFocus: false,
  });
  const s = (value: number) => value * scale;
  const colors = theme.colors;
  const message = (text: string) => (
    <Text
      style={{
        color: colors.foregroundMuted,
        fontSize: s(12),
        lineHeight: s(16),
        paddingHorizontal: s(2),
      }}
    >
      {text}
    </Text>
  );
  if (recaps.isPending) return <ActivityIndicator color={colors.accent} />;
  if (!recaps.data) return message("Could not load recaps. Retrying.");
  if (!recaps.data.days.length) return message("No recaps in the last 7 days.");
  return (
    <View style={{ gap: s(20) }}>
      {recaps.data.days.map((day) => (
        <RecapDaySection key={day.day} day={day} s={s} theme={theme} hues={hues} onOpen={onOpen} />
      ))}
    </View>
  );
}
