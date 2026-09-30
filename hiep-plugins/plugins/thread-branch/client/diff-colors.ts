import type { PluginTheme } from "@getpaseo/plugin";

// Row tints and marker colors copied from Paseo's own diff view.
export const ADD_BACKGROUND = "rgba(46, 160, 67, 0.15)";
export const REMOVE_BACKGROUND = "rgba(248, 81, 73, 0.1)";
const MARKERS = {
  light: { add: "#15803d", remove: "#b91c1c" },
  dark: { add: "#4ade80", remove: "#ef4444" },
};

function isDark(color: string) {
  const hex = color.match(/^#([0-9a-f]{6})/i)?.[1];
  if (!hex) return false;
  const [r, g, b] = [0, 2, 4].map((at) => parseInt(hex.slice(at, at + 2), 16));
  return 0.299 * r + 0.587 * g + 0.114 * b < 128;
}

export function diffMarkers(theme: PluginTheme) {
  return isDark(theme.colors.surface0) ? MARKERS.dark : MARKERS.light;
}
