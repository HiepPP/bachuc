import type { PluginSurfaceProps } from "@getpaseo/plugin/client";
import { Platform, View } from "react-native";
import { ThinkingOrb } from "thinking-orbs";
import { isDarkSurface, orbPreset, sphereCanvas, type OrbStateOption } from "../shared/orb";
import { AgentAvatar } from "./avatar";

/** thinking-orbs draws on a DOM canvas, so native keeps the avatar and host spinner. */
export const orbSupported = Platform.OS === "web";

/** An orb drawn at `size` px; a larger canvas overflows the box evenly on every side. */
export function Orb({
  size,
  state,
  theme,
  color,
}: {
  size: number;
  state: OrbStateOption;
  theme: PluginSurfaceProps["theme"];
  /** Hex or rgb() ink tint; omitted keeps the orb monochrome. */
  color?: string;
}) {
  const orb = orbPreset(size);
  return (
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      style={{ width: size, height: size, alignItems: "center", justifyContent: "center" }}
    >
      <ThinkingOrb
        aria-hidden
        state={state}
        size={orb.size}
        theme={isDarkSurface(theme.colors.surface0) ? "dark" : "light"}
        color={color}
        style={{ transform: `scale(${orb.scale})`, flexShrink: 0 }}
      />
    </View>
  );
}

/**
 * Running-card avatar: the bot sits in a circle whose edge is the orb's silhouette,
 * inside the same footprint as AgentAvatar.
 */
export function OrbAvatar({
  agentId,
  size,
  state,
  opacity,
  theme,
}: {
  agentId: string;
  size: number;
  state: OrbStateOption;
  /** 0–1 avatar opacity under the orb. */
  opacity: number;
  theme: PluginSurfaceProps["theme"];
}) {
  const avatar = size * 0.9;
  const canvas = sphereCanvas(avatar, state);
  return (
    <View style={{ width: size, height: size, alignItems: "center", justifyContent: "center" }}>
      <View
        style={{
          width: avatar,
          height: avatar,
          borderRadius: avatar / 2,
          overflow: "hidden",
          opacity,
          filter: "saturate(0.6)",
        }}
      >
        <AgentAvatar agentId={agentId} size={avatar} />
      </View>
      <View style={{ position: "absolute", left: (size - canvas) / 2, top: (size - canvas) / 2 }}>
        <Orb size={canvas} state={state} theme={theme} />
      </View>
    </View>
  );
}
