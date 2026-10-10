import type { PluginShortcutContribution } from "@getpaseo/plugin/client";
import {
  type KeyboardShortcutInput,
  matchesKeyCombo,
  type ParsedShortcutBinding,
} from "@/keyboard/keyboard-shortcuts";
import { parseChordString } from "@/keyboard/shortcut-string";
import type { InstalledPlugin } from "./types";

export type PluginShortcutResult =
  | { kind: "close"; plugin: InstalledPlugin; shortcut: PluginShortcutContribution }
  | { kind: "press"; plugin: InstalledPlugin; shortcut: PluginShortcutContribution };

/** True when `pathname` shows this plugin's surface, as a page or an overlay. */
export function isPluginSurfacePath(
  pathname: string,
  pluginId: string,
  surfaceId: string,
): boolean {
  return pathname.endsWith(
    `/plugin/${encodeURIComponent(pluginId)}/surface/${encodeURIComponent(surfaceId)}`,
  );
}

// A plugin shortcut fires only when no host shortcut uses the same keys, whatever the host
// shortcut's context, so a plugin can never take a host key. Only plugins of the active host count.
export function resolvePluginShortcut(input: {
  plugins: readonly InstalledPlugin[];
  activeServerId: string | null;
  hostBindings: readonly ParsedShortcutBinding[];
  event: KeyboardShortcutInput;
  isMac: boolean;
  pathname: string;
}): PluginShortcutResult | null {
  const { event, isMac } = input;
  if (input.hostBindings.some((binding) => matchesKeyCombo(binding.parsedChord[0], event, isMac)))
    return null;
  for (const plugin of input.plugins) {
    if (plugin.serverId !== input.activeServerId) continue;
    for (const shortcut of plugin.shortcuts ?? []) {
      const chord = parseChordString(shortcut.combo);
      if (chord.length !== 1 || !matchesKeyCombo(chord[0], event, isMac)) continue;
      const open =
        shortcut.surface !== undefined &&
        isPluginSurfacePath(input.pathname, plugin.id, shortcut.surface);
      return { kind: open ? "close" : "press", plugin, shortcut };
    }
  }
  return null;
}
