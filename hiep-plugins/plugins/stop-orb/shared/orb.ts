/** thinking-orbs ships tuned designs at these sizes; other sizes scale the nearest. */
const PRESETS = [20, 32, 64] as const;
/** The solving sphere's diameter as a fraction of its canvas, from the engine's `R = (size / 2) * k`. */
const SOLVING_SPHERE = 0.82;

/** The preset and scale that draw the solving sphere exactly `diameter` px wide. */
export function solvingOrb(diameter: number): { size: (typeof PRESETS)[number]; scale: number } {
  const canvas = diameter / SOLVING_SPHERE;
  const size = PRESETS.reduce((best, preset) =>
    Math.abs(preset - canvas) < Math.abs(best - canvas) ? preset : best,
  );
  return { size, scale: canvas / size };
}

function channels(color: string): [number, number, number] | null {
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(color.trim())?.[1];
  if (hex) {
    const full = hex.length === 3 ? [...hex].map((digit) => digit + digit).join("") : hex;
    return [0, 2, 4].map((index) => parseInt(full.slice(index, index + 2), 16)) as [
      number,
      number,
      number,
    ];
  }
  const rgb = /^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i.exec(color.trim());
  return rgb ? [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])] : null;
}

/**
 * The orb's `auto` theme cannot see Paseo's theme, so infer it from the surface color.
 * Unknown formats fall back to dark.
 */
export function isDarkSurface(color: string): boolean {
  const rgb = channels(color);
  if (!rgb) return true;
  const [r, g, b] = rgb.map((value) => {
    const c = value / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b < 0.5;
}
