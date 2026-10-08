# TASK-001 Logo concepts with Codex

Group: A (standalone; writes `brand/bachuc/` only)
Class: mechanical

## Brief

Goal: Get three logo concepts for Bachuc from a Codex agent with image generation. The owner picks one, then the agent writes a master SVG mark and a 1024 px PNG.

Change: no Bachuc mark -> [brand/bachuc/mark.svg](brand/bachuc/mark.svg) and [brand/bachuc/mark-1024.png](brand/bachuc/mark-1024.png).

Boundaries:

- Ba Chúc is in Tri Tôn, An Giang, in the Bảy Núi (Seven Mountains) area. The owner has no idea yet, so offer three directions:
  - A sugar palm (thốt nốt) silhouette, the tree of the Bảy Núi plain.
  - A low mountain ridge with a path to it. The path keeps the "walk" idea of the old name, without its look.
  - A letter B made from a ridge line and a palm.
- The mark is flat, has two colors at most, has no text, reads at 16 px, and works on light and dark backgrounds.
- Do not use a butterfly or any shape close to the Paseo mark. Apache 2.0 Section 6 gives no right to the Paseo mark.
- Do not use war or memorial imagery. Ba Chúc has a 1978 massacre memorial, and the mark is about the land.
- Keep the concept images outside the repo, in `/tmp/bachuc-logo/`. Add only the two final files to the repo.
- Image generation makes raster images, and no vectorizer is installed. Write the SVG by hand as simple shapes.
- Done before the autorun Start on 2026-10-08: an `imagegen` Codex agent made the three concepts with the first Prompt below. The owner picked concept 1, the sugar palm (Q-002). Do not make new concepts.
- Concept 1 has a curved trunk and a round fan crown, cream `#F9F0DA` on dark green `#0E4326`, on a rounded square. Its white blotches are broken transparent background; leave them out.
- Simplify the fronds, so the palm still reads at 16 px.

How:

- Look at `/tmp/bachuc-logo/concept-1.png`.
- Write `mark.svg` by hand, per the follow-up Prompt below. You may do this yourself; it needs no image tool.
- Export `mark-1024.png` with the `magick` command in the follow-up Prompt.
- Render the SVG at 16 px and at 1024 px, and look at both.

Files:

- [brand/bachuc/mark.svg](brand/bachuc/mark.svg) (new master mark)
- [brand/bachuc/mark-1024.png](brand/bachuc/mark-1024.png) (new 1024 px export)

Expected result:

- Three concept PNGs exist in `/tmp/bachuc-logo/`, one per direction.
- Q-002 holds the owner's pick.
- `mark.svg` uses only `path`, `rect`, `circle`, `ellipse`, or `polygon`, has no embedded raster, and matches the pick.

Anti-goal: repo growth from this TASK stays at or below 1024 KiB; tripwire; read with `du -sk brand/bachuc` at completion.

Prompt:

```text
Use your image generation tool. Make three app icon concepts for a desktop app named Bachuc. Save each as a 1024x1024 PNG: /tmp/bachuc-logo/concept-1.png, /tmp/bachuc-logo/concept-2.png, /tmp/bachuc-logo/concept-3.png.
Bachuc is named after Ba Chúc, a place in Tri Tôn, An Giang, Vietnam, in the Bảy Núi (Seven Mountains) area.
1. A sugar palm (thốt nốt) silhouette.
2. A low mountain ridge with a path that leads to it.
3. A letter B formed by a ridge line and a palm.
Rules: flat vector style, two colors at most, no text, centered on a rounded square, clear at 16 px, works on light and dark backgrounds. No butterflies. No war, memorial, or skull imagery.
Reply with the three file paths and one line per concept.
```

Follow-up Prompt:

```text
The owner picked concept <N>. Write /Users/hiep/Projects/paseo/brand/bachuc/mark.svg by hand as simple SVG shapes (path, rect, circle, ellipse, polygon). Use viewBox 0 0 1024 1024, no embedded images, and two fill colors at most. Then run from /Users/hiep/Projects/paseo: magick -background none brand/bachuc/mark.svg -resize 1024x1024 brand/bachuc/mark-1024.png. Do not edit any other file. Reply with both paths.
```

## Verify

- `ls /tmp/bachuc-logo/concept-1.png /tmp/bachuc-logo/concept-2.png /tmp/bachuc-logo/concept-3.png` -> three files.
- `rg -c "<image" brand/bachuc/mark.svg` -> no match (exit 1).
- `magick identify -format "%wx%h" brand/bachuc/mark-1024.png` -> `1024x1024`.
- Human check: the owner confirms that the SVG matches the picked concept at 16 px and at 1024 px.
- Anti-goal: `du -sk brand/bachuc` -> 1024 or less.
