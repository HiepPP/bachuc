import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

export const projectColors = defineSettings({
  id: "project-colors",
  scope: "host",
  version: 1,
  schema: z.object({ hues: z.record(z.string(), z.number().int().min(0).max(359)).default({}) }),
});

export function allocateColors(existing: Record<string, number>, ids: readonly string[]) {
  const missing = [...new Set(ids)].filter((id) => !Object.hasOwn(existing, id)).sort();
  if (!missing.length) return existing;
  const result = { ...existing };
  const used = Object.values(existing);
  for (const id of missing) {
    let bestHue = 0;
    let bestDistance = -1;
    for (let hue = 0; hue < 360; hue++) {
      const distance = used.reduce((minimum, other) => {
        const difference = Math.abs(hue - other);
        return Math.min(minimum, difference, 360 - difference);
      }, 180);
      if (distance > bestDistance) {
        bestHue = hue;
        bestDistance = distance;
      }
    }
    result[id] = bestHue;
    used.push(bestHue);
  }
  return result;
}
