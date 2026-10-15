import type { PluginButtonIcon, PluginButtonIconProps } from "@getpaseo/plugin/client";
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

/** The branch pill icon: host-muted when in sync, tinted when commits wait to push or pull. */
export const branchIcons: Record<SyncState, PluginButtonIcon> = {
  synced: "GitBranch",
  ahead: tinted("GitBranch", (colors) => colors.accent),
  behind: tinted("GitBranch", (colors) => colors.statusWarning),
};
