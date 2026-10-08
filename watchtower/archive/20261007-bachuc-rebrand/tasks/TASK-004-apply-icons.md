# TASK-004 Apply the mark to icons and logo

Group: D (`packages/app/assets/images/`, `packages/app/public/` images, `packages/desktop/assets/` icons, `packages/app/src/components/icons/paseo-logo.tsx`)
Class: mechanical

## Brief

Goal: Every app, web, and desktop icon and the in-app logo show the Bachuc mark from TASK-001. The Paseo butterfly appears nowhere in the app.

Change: butterfly icons and logo -> the mark in [brand/bachuc/mark.svg](brand/bachuc/mark.svg), at the same file names and pixel sizes.

Boundaries:

- Needs the owner's pick in Q-002 and the files from TASK-001.
- Keep every file name and pixel size, so no config or code path changes.
- Follows [ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md): keep the component name `PaseoLogo` and its props. Change only its drawing.
- [packages/app/src/hooks/use-favicon-status.ts](packages/app/src/hooks/use-favicon-status.ts) uses favicon variants for `running` and `attention`. Copy the status dot from each old SVG onto the new mark.
- Keep `icon-dev.png` visibly different from `icon.png`, as the old pair is.
- Replace `butterfly-green.svg` and `butterfly-white.svg` with the mark only if code still imports them. Delete them if nothing imports them.
- Use `magick` for PNG sizes and `.ico`, and `iconutil` for `.icns` from an `.iconset` folder.
- Mobile-only images (`android-icon-foreground.png`, `notification-icon.png`, `splash-icon.png`) are replaced too, so no butterfly stays in the repo assets. Mobile is still not rebuilt.

How:

- Record the name, size, and dimensions of each file below before you change it.
- Export each PNG from `mark.svg` at its old dimensions.
- Build `icon.icns` and `icon.ico` from the exports.
- Write the favicon SVG variants from `mark.svg` plus the old status dots.
- Port the shapes of `mark.svg` into `paseo-logo.tsx`, using the elements the file already uses.

Files:

- [packages/app/assets/images](packages/app/assets/images) (`icon.png`, `favicon*.png`, `favicon*.svg`, `splash-icon.png`, `notification-icon.png`, `android-icon-foreground.png`, `butterfly-green.svg`, `butterfly-white.svg`)
- [packages/app/public](packages/app/public) (`apple-touch-icon.png`, `pwa-icon-192.png`, `pwa-icon-512.png`)
- [packages/desktop/assets](packages/desktop/assets) (`icon.icns`, `icon.ico`, `icon.png`, `icon-dev.png`, `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`)
- [packages/app/src/components/icons/paseo-logo.tsx](packages/app/src/components/icons/paseo-logo.tsx) (new drawing, same API)

Expected result:

- Each replaced PNG keeps its old dimensions.
- The welcome screen, the startup splash, the open project screen, and the browser tab favicon show the mark.
- The favicon still shows the running and attention dots.
- The live desktop Dock icon shows the mark.

Anti-goal: the total size of the replaced asset files grows by 25 percent or less over the baseline; drift gauge; read with `du -ck <files>` before changes and at completion.

## Verify

- For each PNG: `magick identify -format "%f %wx%h\n" <file>` -> the same dimensions as the baseline in the outcome.
- `iconutil -c iconset packages/desktop/assets/icon.icns -o /tmp/bachuc.iconset` -> exit 0, and the images show the mark.
- `rg -n "butterfly-(green|white)" packages/app/src` -> no match, or each match loads a file that now holds the mark.
- `npm run typecheck` -> exit 0.
- Human check on live: the welcome screen, the startup splash, the open project screen, the tab favicon in its three states, and the Dock icon show the mark.
- Anti-goal: `du -ck <replaced files>` -> total is at most 1.25 times the baseline.
