import type { Theme } from "@/styles/theme";

/** The app theme as `--paseo-*` custom properties inside a plugin HTML frame. */
export function htmlFrameCssVariables(theme: Theme): Record<string, string> {
  const { colors, fontFamily } = theme;
  return {
    "paseo-background": colors.surface0,
    "paseo-surface": colors.surface1,
    "paseo-foreground": colors.foreground,
    "paseo-muted": colors.foregroundMuted,
    "paseo-border": colors.border,
    "paseo-accent": colors.accent,
    "paseo-success": colors.statusSuccess,
    "paseo-warning": colors.statusWarning,
    "paseo-danger": colors.statusDanger,
    "paseo-font-ui": fontFamily.ui,
    "paseo-font-mono": fontFamily.mono,
  };
}
