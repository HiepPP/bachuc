import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

/** thinking-orbs ships separately tuned designs at these sizes; other sizes scale the nearest. */
const ORB_PRESETS = [20, 32, 64] as const;

export function orbPreset(target: number): { size: (typeof ORB_PRESETS)[number]; scale: number } {
  const size = ORB_PRESETS.reduce((best, preset) =>
    Math.abs(preset - target) < Math.abs(best - target) ? preset : best,
  );
  return { size, scale: target / size };
}

/**
 * Silhouette diameter as a fraction of the canvas, from each engine mode's `R = (size / 2) * k`.
 * breathing (ring) and shaping (morph) are not round, so an avatar edge cannot follow them.
 */
const ORB_SPHERE = {
  solving: 0.82,
  working: 0.82,
  searching: 0.82,
  listening: 0.874,
  weaving: 0.76,
  composing: 0.78,
  connecting: 0.8,
} as const;

export type OrbStateOption = keyof typeof ORB_SPHERE;
export const ORB_STATES = Object.keys(ORB_SPHERE) as [OrbStateOption, ...OrbStateOption[]];

/** Canvas size whose orb silhouette has the given diameter. */
export function sphereCanvas(diameter: number, state: OrbStateOption): number {
  return diameter / ORB_SPHERE[state];
}

export const orbSettings = defineSettings({
  id: "thinking-orb",
  scope: "host",
  version: 1,
  schema: z.object({
    enabled: z.boolean().default(true),
    state: z.enum(ORB_STATES).default("solving"),
    avatarOpacity: z.number().int().min(20).max(100).multipleOf(10).default(100),
  }),
});

export type OrbSettings = z.infer<typeof orbSettings.schema>;

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
 * Plugin themes expose colors but no light/dark flag, and the orb's `auto` theme cannot see
 * Paseo's theme, so infer the mode from the surface. Unknown formats fall back to dark.
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
