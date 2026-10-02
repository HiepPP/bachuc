import type { PluginSurfaceProps } from "@getpaseo/plugin/client";
import { isDarkSurface } from "../shared/orb";

type Theme = Pick<PluginSurfaceProps["theme"], "colors">;

/** Card tint and border in the project's saved hue; neutral until a hue is assigned. */
export function repoSurface(hue: number | undefined, theme: Theme) {
  const { colors } = theme;
  if (hue === undefined) return { backgroundColor: colors.surface1, borderColor: colors.border };
  const dark = isDarkSurface(colors.surface1);
  return {
    backgroundColor: `hsl(${hue}, ${dark ? 14 : 38}%, ${dark ? 14 : 97}%)`,
    borderColor: `hsl(${hue}, ${dark ? 22 : 30}%, ${dark ? 29 : 84}%)`,
  };
}

/** Project mark fill, as in the sidebar. */
export function repoMark(hue: number | undefined, theme: Theme) {
  return hue === undefined ? theme.colors.foregroundMuted : `hsl(${hue}, 42%, 58%)`;
}

/**
 * Repo name inside the card, in the card's own hue. Light lightness 31 keeps 4.5:1 for every
 * hue on the tint and on the hovered and pressed card; 32 falls short once the card is hovered.
 */
export function repoLabel(hue: number | undefined, theme: Theme) {
  if (hue === undefined) return theme.colors.foregroundMuted;
  return `hsl(${hue}, 45%, ${isDarkSurface(theme.colors.surface1) ? 75 : 31}%)`;
}

/** Line between a parent card and its subagents: the card border, one step quieter in light. */
export function repoDivider(hue: number | undefined, theme: Theme) {
  if (hue === undefined) return theme.colors.border;
  return isDarkSurface(theme.colors.surface1)
    ? repoSurface(hue, theme).borderColor
    : `hsl(${hue}, 30%, 88%)`;
}
