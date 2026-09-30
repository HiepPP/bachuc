import type { PluginButtonIconProps } from "@getpaseo/plugin/client";
import * as nativeUi from "@getpaseo/plugin/client/react-native";
import type { ComponentType } from "react";

export type SyncState = "synced" | "ahead" | "behind";

type Colors = PluginButtonIconProps["theme"]["colors"];

// Module-level components keep a stable identity so poll updates never remount the icon.
function tinted(
  name: string,
  pick: (colors: Colors) => string,
): ComponentType<PluginButtonIconProps> {
  return function SyncIcon({ theme, size }: PluginButtonIconProps) {
    // Namespace access: the host supplies `Icon` at runtime; Node tests never render it.
    const Icon = nativeUi.Icon;
    return <Icon name={name} size={size} color={pick(theme.colors)} />;
  };
}

export const syncIcons: Record<SyncState, ComponentType<PluginButtonIconProps>> = {
  synced: tinted("CircleCheck", (colors) => colors.statusSuccess),
  ahead: tinted("CloudUpload", (colors) => colors.accent),
  behind: tinted("CloudDownload", (colors) => colors.statusWarning),
};
