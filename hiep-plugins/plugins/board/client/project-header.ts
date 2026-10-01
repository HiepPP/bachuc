export function projectInitial(name: string) {
  return Array.from(name.trim())[0]?.toUpperCase() ?? "?";
}

export interface ChipColors {
  background: string;
  border: string;
  markBackground: string;
  markText: string;
}

/** Board hue once Board has assigned one, otherwise a neutral tint from the theme. */
export function chipColors(
  hue: number | undefined,
  theme: { surface2: string; border: string; foregroundMuted: string; surface0: string },
): ChipColors {
  if (hue === undefined)
    return {
      background: theme.surface2,
      border: theme.border,
      markBackground: theme.foregroundMuted,
      markText: theme.surface0,
    };
  return {
    background: `hsla(${hue}, 60%, 72%, 0.2)`,
    border: `hsla(${hue}, 60%, 72%, 0.8)`,
    markBackground: `hsl(${hue}, 42%, 58%)`,
    markText: "#fff",
  };
}
