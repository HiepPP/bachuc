import assert from "node:assert/strict";
import test from "node:test";
import { repoDivider, repoLabel, repoMark, repoSurface } from "../client/repo-color";

type Rgb = [number, number, number];
// Paseo light and dark surfaces (packages/app/src/styles/theme.ts).
const light = { colors: { surface1: "#fafafa", surface2: "#f4f4f5", border: "#e4e4e7" } } as any;
// Zinc dark, and the dark theme with the lightest surfaces.
const dark = { colors: { surface1: "#1f1f22", surface2: "#27272a", border: "#27272a" } } as any;
const dim = { colors: { surface1: "#2f333d", surface2: "#383c48", border: "#353a47" } } as any;

function rgb(color: string): Rgb {
  const hex = /^#([0-9a-f]{6})$/i.exec(color);
  if (hex) return [0, 2, 4].map((at) => parseInt(hex[1].slice(at, at + 2), 16) / 255) as Rgb;
  const [h, s, l] = /^hsl\((\d+), (\d+)%, (\d+)%\)$/.exec(color)!.slice(1).map(Number);
  const a = (s / 100) * Math.min(l / 100, 1 - l / 100);
  const channel = (n: number) => {
    const k = (n + h / 30) % 12;
    return l / 100 - a * Math.max(-1, Math.min(k - 3, 9 - k, 1));
  };
  return [channel(0), channel(8), channel(4)];
}
function luminance(color: Rgb) {
  const [r, g, b] = color.map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function contrast(a: Rgb, b: Rgb) {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (high + 0.05) / (low + 0.05);
}
const mix = (a: Rgb, b: Rgb, t: number) => a.map((c, i) => c * (1 - t) + b[i] * t) as Rgb;

test("the repo label keeps 4.5:1 on the card tint, hovered, and pressed for every hue", () => {
  for (const theme of [light, dark, dim]) {
    const surface2 = rgb(theme.colors.surface2);
    for (let hue = 0; hue < 360; hue += 1) {
      const text = rgb(repoLabel(hue, theme));
      const tint = rgb(repoSurface(hue, theme).backgroundColor);
      // The card's hover wash is surface2 at 55%; a press fills it with surface2.
      for (const surface of [tint, mix(tint, surface2, 0.55), surface2])
        assert.ok(contrast(text, surface) >= 4.5, `hue ${hue}: ${contrast(text, surface)}`);
    }
  }
});

test("projects without a hue stay neutral", () => {
  const theme = { colors: { ...light.colors, foregroundMuted: "#71717a" } } as any;
  assert.equal(repoLabel(undefined, theme), "#71717a");
  assert.equal(repoMark(undefined, theme), "#71717a");
  assert.equal(repoDivider(undefined, theme), "#e4e4e7");
  assert.deepEqual(repoSurface(undefined, theme), {
    backgroundColor: "#fafafa",
    borderColor: "#e4e4e7",
  });
  assert.equal(repoSurface(200, theme).backgroundColor, "hsl(200, 38%, 97%)");
  assert.equal(repoMark(200, theme), "hsl(200, 42%, 58%)");
});
