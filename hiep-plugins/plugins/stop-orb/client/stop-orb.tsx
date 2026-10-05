import type { PluginComposerStopButtonProps } from "@getpaseo/plugin/client";
import { useState } from "react";
import { View } from "react-native";
import { ThinkingOrb } from "thinking-orbs";
import { isDarkSurface, solvingOrb } from "../shared/orb";

// The red of Paseo's own stop button (palette red 600), the same in every theme.
const STOP_RED = "#dc2626";

/**
 * The solving orb, its sphere as wide as the composer's other icons. It turns red on hover to
 * show that a press stops the agent, and freezes while the agent stops.
 */
export function StopOrb({ size, iconSize, cancelling, theme }: PluginComposerStopButtonProps) {
  const [hovered, setHovered] = useState(false);
  const orb = solvingOrb(iconSize);
  return (
    // Hover sits on a plain View, apart from the host's pressable; see docs/hover.md.
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      style={{ width: size, height: size, alignItems: "center", justifyContent: "center" }}
    >
      <ThinkingOrb
        aria-hidden
        state="solving"
        size={orb.size}
        paused={cancelling}
        // The 20 px preset draws fine dots; bolder ones match the weight of the icon strokes.
        dotSize={1.4}
        theme={isDarkSurface(theme.colors.surface0) ? "dark" : "light"}
        color={hovered && !cancelling ? STOP_RED : undefined}
        style={{ transform: `scale(${orb.scale})`, flexShrink: 0 }}
      />
    </View>
  );
}
