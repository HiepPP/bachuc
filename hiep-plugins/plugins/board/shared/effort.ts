import { isDarkSurface } from "./orb";

/**
 * Hue rises with reasoning depth. Lightness differs per theme so each tag keeps at least 4.5:1
 * contrast on every Paseo card surface, including the lightest dark theme.
 */
const LEVELS = [
  { match: /max|ultra/, hue: 330, saturation: 70, light: 42, dark: 77 },
  { match: /extra|xhigh/, hue: 15, saturation: 85, light: 38, dark: 72 },
  { match: /high/, hue: 35, saturation: 90, light: 30, dark: 55 },
  { match: /medium/, hue: 220, saturation: 75, light: 45, dark: 76 },
  { match: /low|minimal/, hue: 185, saturation: 70, light: 28, dark: 52 },
];

/** Tag color for an effort id or label on `surface`; null for unknown levels. */
export function effortColor(effort: string, surface: string): string | null {
  const level = effort.toLowerCase().replace(/[\s_-]/g, "");
  const tone = LEVELS.find((item) => item.match.test(level));
  if (!tone) return null;
  const lightness = isDarkSurface(surface) ? tone.dark : tone.light;
  return `hsl(${tone.hue}, ${tone.saturation}%, ${lightness}%)`;
}
