// The turn step list colors its Explore, Edit, and Verify steps with fixed hues, a documented
// design exception (docs/design.md §13, ADR-0020). The text and border tones live in the theme
// (`theme.turnLabels`), because on web Unistyles hands `StyleSheet.create` a theme of CSS
// variables, so a style factory cannot branch on `theme.colorScheme`.

type TurnLabel = "explore" | "edit" | "verify";
type ColorScheme = "light" | "dark";

/** The one place that holds the label hues (oklch). */
export const LABEL_HUES: Record<TurnLabel, number> = { explore: 155, edit: 55, verify: 300 };

export interface TurnLabelColors {
  /** Strip segment. Not text, so it is the same in both schemes. */
  segment: string;
  /** Label text and number. */
  text: string;
  /** Number border. */
  border: string;
}

/**
 * Text and border tone (oklch lightness, chroma) per scheme. No single text tone clears 4.5:1
 * on both a white and a dark surface, so only lightness moves; the hue stays.
 */
const LABEL_TONES: Record<ColorScheme, { text: [number, number]; border: [number, number] }> = {
  light: { text: [0.52, 0.11], border: [0.8, 0.06] },
  dark: { text: [0.75, 0.1], border: [0.45, 0.07] },
};

/**
 * oklch to an rgb() string. Native has no oklch parser, so the design formulas
 * run here once and every platform gets plain rgb.
 */
function oklchToRgb(lightness: number, chroma: number, hue: number): string {
  const radians = (hue * Math.PI) / 180;
  const a = chroma * Math.cos(radians);
  const b = chroma * Math.sin(radians);
  const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const linear = [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
  const channels = linear.map((value) => {
    const clamped = Math.min(1, Math.max(0, value));
    const encoded = clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * clamped ** (1 / 2.4) - 0.055;
    return Math.round(encoded * 255);
  });
  return `rgb(${channels.join(", ")})`;
}

function buildLabelColors(hue: number, scheme: ColorScheme): TurnLabelColors {
  const { text, border } = LABEL_TONES[scheme];
  return {
    segment: oklchToRgb(0.74, 0.09, hue),
    text: oklchToRgb(text[0], text[1], hue),
    border: oklchToRgb(border[0], border[1], hue),
  };
}

function buildSchemeColors(scheme: ColorScheme): Record<TurnLabel, TurnLabelColors> {
  return {
    explore: buildLabelColors(LABEL_HUES.explore, scheme),
    edit: buildLabelColors(LABEL_HUES.edit, scheme),
    verify: buildLabelColors(LABEL_HUES.verify, scheme),
  };
}

const LABEL_COLORS: Record<ColorScheme, Record<TurnLabel, TurnLabelColors>> = {
  light: buildSchemeColors("light"),
  dark: buildSchemeColors("dark"),
};

export function turnLabelColors(label: TurnLabel, colorScheme: ColorScheme): TurnLabelColors {
  return LABEL_COLORS[colorScheme][label];
}

/** The theme's `turnLabels` value for one scheme. */
export function turnLabelTheme(colorScheme: ColorScheme): Record<TurnLabel, TurnLabelColors> {
  return LABEL_COLORS[colorScheme];
}
