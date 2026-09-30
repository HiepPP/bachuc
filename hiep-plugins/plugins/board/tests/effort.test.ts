import assert from "node:assert/strict";
import test from "node:test";
import { effortColor } from "../shared/effort";

// surface1 and surface2 of each built-in Paseo theme; tags render on both.
const SURFACES = {
  light: ["#fafafa", "#f4f4f5"],
  dark: ["#1E2120", "#272A29", "#1f1f22", "#27272a", "#1c1e27", "#252731"],
  warm: ["#262523", "#2f2d2b"],
  oneDark: ["#2f333d", "#383c48"],
  black: ["#0a0a0a", "#111111"],
};
const LEVELS = ["low", "medium", "high", "xhigh", "Extra High", "max", "ultracode"];

function rgb(color: string): number[] {
  const hsl = /^hsl\((\d+), (\d+)%, (\d+)%\)$/.exec(color);
  if (hsl) {
    const [h, s, l] = [Number(hsl[1]), Number(hsl[2]) / 100, Number(hsl[3]) / 100];
    const f = (n: number) => {
      const k = (n + h / 30) % 12;
      return l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    };
    return [f(0), f(8), f(4)];
  }
  return [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16) / 255);
}
function luminance(color: string) {
  const [r, g, b] = rgb(color).map((c) =>
    c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function contrast(a: string, b: string) {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (high + 0.05) / (low + 0.05);
}

test("every effort tag reaches 4.5:1 on every theme surface", () => {
  for (const [theme, surfaces] of Object.entries(SURFACES)) {
    for (const level of LEVELS) {
      // Tags pick lightness from surface2; both card surfaces must still pass.
      const color = effortColor(level, surfaces.at(-1)!)!;
      for (const surface of surfaces) {
        const ratio = contrast(color, surface);
        assert.ok(ratio >= 4.5, `${level} on ${theme} ${surface}: ${ratio.toFixed(2)}`);
      }
    }
  }
});

test("keeps hues by depth and leaves unknown levels to the caller", () => {
  assert.match(effortColor("high", "#f4f4f5")!, /^hsl\(35,/);
  assert.match(effortColor("Extra High", "#272A29")!, /^hsl\(15,/);
  assert.equal(effortColor("off", "#f4f4f5"), null);
});
