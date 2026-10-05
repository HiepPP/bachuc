import type { PluginComposerStopButtonProps } from "@getpaseo/plugin/client";
import { useState } from "react";
import { View } from "react-native";
import { ThinkingOrb } from "thinking-orbs";
import { isDarkSurface, solvingOrb } from "../shared/orb";

// The red of Paseo's own stop button (palette red 600), the same in every theme.
const STOP_RED = "#dc2626";
/** How much faster the orb spins under the pointer. */
const HOVER_SPEED = 1.8;

/**
 * The solving orb, its sphere as wide as the composer's other icons, always in the stop button's
 * red instead of the library's grayscale ink. On hover it gains the composer's icon-button
 * background and spins faster; it freezes while the agent stops.
 */
export function StopOrb({ size, iconSize, cancelling, theme }: PluginComposerStopButtonProps) {
  const [hovered, setHovered] = useState(false);
  const active = hovered && !cancelling;
  const orb = solvingOrb(iconSize);
  return (
    // Hover sits on a plain View, apart from the host's pressable; see docs/hover.md.
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      style={{
        width: size,
        height: size,
        borderRadius: size / 2,
        alignItems: "center",
        justifyContent: "center",
        // The same hover background as the composer's other icon buttons.
        backgroundColor: active ? theme.colors.surface2 : "transparent",
      }}
    >
      <ThinkingOrb
        aria-hidden
        state="solving"
        size={orb.size}
        paused={cancelling}
        speed={active ? HOVER_SPEED : 1}
        // The 20 px preset draws fine dots; bolder ones match the weight of the icon strokes.
        dotSize={1.4}
        theme={isDarkSurface(theme.colors.surface0) ? "dark" : "light"}
        color={STOP_RED}
        style={{ transform: `scale(${orb.scale})`, flexShrink: 0 }}
      />
    </View>
  );
}
