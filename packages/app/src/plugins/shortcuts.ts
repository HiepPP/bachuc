import type { PluginShortcutContribution } from "@getpaseo/plugin/client";
import {
  type KeyboardShortcutInput,
  matchesKeyCombo,
  type ParsedShortcutBinding,
} from "@/keyboard/keyboard-shortcuts";
import { parseChordString } from "@/keyboard/shortcut-string";
import { hostIdFromPathname } from "./routes";
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
// shortcut's context, so a plugin can never take a host key. Like plugin sidebar rows, an active
// host pins the plugin to that host. In "All machines" mode (no active host), the host of the
// current route wins, and outside any host route the first plugin with the combo does.
export function resolvePluginShortcut(input: {
  plugins: readonly InstalledPlugin[];
  activeServerId: string | null;
  hostBindings: readonly ParsedShortcutBinding[];
  event: KeyboardShortcutInput;
  isMac: boolean;
  pathname: string;
}): PluginShortcutResult | null {
  const { event, isMac } = input;
  // An unassigned host shortcut has an empty chord and takes no keys.
  const hostTakes = input.hostBindings.some((binding) => {
    const first = binding.parsedChord[0];
    return first !== undefined && matchesKeyCombo(first, event, isMac);
  });
  if (hostTakes) return null;
  const hostId = input.activeServerId ?? hostIdFromPathname(input.pathname);
  for (const plugin of input.plugins) {
    if (hostId !== null && plugin.serverId !== hostId) continue;
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
