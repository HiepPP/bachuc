import type { PluginTheme } from "@getpaseo/plugin";
import { Pressable, Text } from "react-native";

// The Watchtower TASK a thread served (ADR-0012). Pressing it opens Watchtower at that TASK.
export function TaskChip({
  id,
  theme,
  scale,
  compact = false,
  onPress,
}: {
  id: string;
  theme: PluginTheme;
  scale: number;
  compact?: boolean;
  onPress?: () => void;
}) {
  const s = (value: number) => value * scale;
  const chip = (
    <Text
      style={{
        paddingHorizontal: s(compact ? 4 : 6),
        paddingVertical: s(1),
        borderRadius: s(4),
        overflow: "hidden",
        backgroundColor: theme.colors.foreground,
        color: theme.colors.surface0,
        fontSize: s(compact ? 9.5 : 10.5),
        lineHeight: s(compact ? 13 : 15),
        fontWeight: "700",
        letterSpacing: 0.3,
      }}
    >
      {id}
    </Text>
  );
  if (!onPress) return chip;
  return (
    <Pressable
      accessibilityRole="link"
      accessibilityLabel={`Open ${id} in Watchtower`}
      onPress={onPress}
      hitSlop={s(4)}
      style={({ pressed }) => ({ flexShrink: 0, opacity: pressed ? 0.7 : 1 })}
    >
      {chip}
    </Pressable>
  );
}
