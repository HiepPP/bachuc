import type { PluginSurfaceProps } from "@getpaseo/plugin/client";
import { Pressable, Text, View } from "react-native";
import {
  BOARD_SIZE_DEFAULT,
  BOARD_SIZE_MAX,
  BOARD_SIZE_MIN,
  BOARD_SIZE_STEP,
} from "../shared/board-size";

export function BoardSizeControl({
  size,
  onChange,
  theme,
}: {
  size: number;
  onChange: (size: number) => void;
  theme: PluginSurfaceProps["theme"];
}) {
  const colors = theme.colors;
  return (
    <View style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <Text style={{ color: colors.foregroundMuted, fontSize: 13 }}>UI size</Text>
      <View
        style={{
          flexDirection: "row",
          borderWidth: 1,
          borderColor: colors.border,
          borderRadius: 10,
          overflow: "hidden",
          backgroundColor: colors.surface1,
        }}
      >
        {[
          {
            label: "Decrease Board UI size",
            text: "−",
            value: size - BOARD_SIZE_STEP,
            disabled: size <= BOARD_SIZE_MIN,
          },
          {
            label: `Board UI size ${size}%. Reset to ${BOARD_SIZE_DEFAULT}%`,
            text: `${size}%`,
            value: BOARD_SIZE_DEFAULT,
            disabled: false,
          },
          {
            label: "Increase Board UI size",
            text: "+",
            value: size + BOARD_SIZE_STEP,
            disabled: size >= BOARD_SIZE_MAX,
          },
        ].map((item, index) => (
          <Pressable
            key={index}
            accessibilityRole="button"
            accessibilityLabel={item.label}
            accessibilityHint={index === 1 ? "Restore default Board size" : "Affects only Board"}
            accessibilityState={{ disabled: item.disabled }}
            disabled={item.disabled}
            onPress={() => onChange(item.value)}
            style={({ pressed }) => ({
              width: index === 1 ? 62 : 44,
              height: 44,
              flexShrink: 0,
              alignItems: "center",
              justifyContent: "center",
              borderLeftWidth: index ? 1 : 0,
              borderColor: colors.border,
              backgroundColor: pressed ? colors.surface2 : colors.surface1,
              opacity: item.disabled ? 0.35 : 1,
            })}
          >
            <Text
              style={{
                color: colors.foreground,
                fontSize: index === 1 ? 13 : 21,
                fontWeight: "500",
              }}
            >
              {item.text}
            </Text>
          </Pressable>
        ))}
      </View>
    </View>
  );
}
