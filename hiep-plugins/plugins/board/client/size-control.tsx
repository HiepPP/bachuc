import type { PluginSurfaceProps } from "@getpaseo/plugin/client";
import { useState } from "react";
import { Pressable, Text, View } from "react-native";
import { focusRing, keyboardFocus } from "./focus";
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
  // Index of the button holding keyboard focus; clicks do not draw the ring.
  const [focused, setFocused] = useState<number | null>(null);
  return (
    // Bare stepper: no frame or label, so the toolbar stays quieter than the Board content.
    <View
      accessibilityLabel="Board UI size"
      style={{ flexDirection: "row", alignItems: "center", gap: 2 }}
    >
      <>
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
            onFocus={(event) => setFocused(keyboardFocus(event) ? index : null)}
            onBlur={() => setFocused(null)}
            hitSlop={{ top: 8, bottom: 8 }}
            style={({ pressed }) => ({
              width: index === 1 ? 48 : 28,
              height: 28,
              flexShrink: 0,
              alignItems: "center",
              justifyContent: "center",
              borderRadius: 6,
              backgroundColor: pressed ? colors.surface2 : "transparent",
              opacity: item.disabled ? 0.35 : 1,
              ...(focused === index ? focusRing(colors.accent) : null),
            })}
          >
            <Text
              style={{
                color: index === 1 ? colors.foreground : colors.foregroundMuted,
                fontSize: index === 1 ? 12.5 : 17,
                lineHeight: index === 1 ? 17 : 20,
                fontWeight: "500",
                fontVariant: ["tabular-nums"],
              }}
            >
              {item.text}
            </Text>
          </Pressable>
        ))}
      </>
    </View>
  );
}
