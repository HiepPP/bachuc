# TASK-004 Outcome

## Outcome

Status: DONE (auto: manual checks pending)

Changed:

- [packages/app/assets/images](packages/app/assets/images): `icon.png`, `favicon.png`, `splash-icon.png`, and the six `favicon-*` PNGs show the mark as a full-bleed rounded square, as the old black squares did. `notification-icon.png` and `android-icon-foreground.png` show a white palm on transparent, at the old glyph size.
- The six `favicon-*.svg` files draw the mark and keep the old status dots: blue `#3b82f6` for running, green `#22c55e` for attention. Dark and light stay identical, as before. The favicon PNGs are rendered from these SVGs.
- [packages/app/public](packages/app/public): `apple-touch-icon.png`, `pwa-icon-192.png`, and `pwa-icon-512.png` show the full-bleed mark.
- [packages/desktop/assets](packages/desktop/assets): `icon.png`, the four size PNGs, `icon.icns`, and `icon.ico` show the mark on the macOS grid (824 px body at offset 100 in 1024), as the old desktop icons did. `icon-dev.png` adds a cream `</>` badge at the bottom right.
- [packages/app/src/components/icons/paseo-logo.tsx](packages/app/src/components/icons/paseo-logo.tsx): draws the palm from [brand/bachuc/mark.svg](brand/bachuc/mark.svg) without its rounded square, one `Path` per shape. The name `PaseoLogo`, its props, and the single-color fill stay (ADR-0004), so the splash mask still works.
- Deleted `butterfly-green.svg` and `butterfly-white.svg`. Nothing imported them.

Contract:

- Baseline dimensions: `icon.png` 1024x1024, the seven favicon PNGs 48x48, `splash-icon.png` 200x200, `notification-icon.png` 96x96, `android-icon-foreground.png` 1024x1024, `apple-touch-icon.png` 180x180, `pwa-icon-192.png` 192x192, `pwa-icon-512.png` 512x512. Desktop: `icon.png` 512x512, `icon-dev.png` 1024x1024, `32x32.png`, `64x64.png`, `128x128.png`, and `128x128@2x.png` 256x256. `icon.ico` held PNG entries at 16, 24, 32, 48, 64, 128, and 256. `icon.icns` held the ten standard sizes.
- Q-006, DEFAULTED: the spec says to keep `icon-dev.png` different "as the old pair is". The old desktop `icon.png` and `icon-dev.png` were the same orange "Paseo Dev" image (RMSE 0.007). The default marks the dev icon with a `</>` badge, so live and release still differ in the Dock.
- Tools: `magick` here has no SVG delegate (`rsvg-convert` is missing), so the PNGs are rendered with `sharp` from `node_modules`, which bundles librsvg 2.61. `magick` writes only BMP entries into `.ico` (372 KB), so a short script packed the PNG exports into `icon.ico`. `iconutil` built `icon.icns`.
- GitNexus `impact` on `PaseoLogo`: CRITICAL by reach (31 dependents, the welcome and open project screens). The API does not change, so no caller changes. No symbol-level impact applies to the image files.

Verified:

- `magick identify -format "%f %wx%h\n"` on each replaced PNG -> each matches the baseline above.
- `iconutil -c iconset packages/desktop/assets/icon.icns -o /tmp/bachuc.iconset` -> exit 0, ten images, each shows the mark.
- `magick identify packages/desktop/assets/icon.ico` -> seven PNG entries, 16 to 256.
- `rg -n "butterfly-(green|white)" packages/app/src` -> no match.
- A contact sheet of all replaced icons and a render of the new `PaseoLogo` paths show the mark, centered.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/components/icons/paseo-logo.tsx` -> 0 warnings, 0 errors.
- Plan Verify: the license diff exits 0, and the plugin diff has no output.
- UNVERIFIED (autonomous run): the human check on live (welcome screen, startup splash, open project screen, the tab favicon in its three states, and the Dock icon). The run does not start or restart live.

Anti-goal:

- Before changes: `du -ck <replaced files>` -> 2756 KB, 2026-10-08 10:46. The baseline includes the two deleted butterfly SVGs.
- Final: `du -ck <replaced files>` -> 636 KB, 10:50.
- Result: PASS (0.23 of the baseline, limit 1.25).

Lessons:

- `magick` on this machine cannot render SVG and writes `.ico` entries as BMP. Render SVG with `sharp` from `node_modules`, and pack PNG files into `.ico` by hand.
- The desktop `icon.png` and `icon-dev.png` were identical before this TASK. The dev icon now carries a badge.
