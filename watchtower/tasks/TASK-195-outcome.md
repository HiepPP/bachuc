# TASK-195 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs`: new `PLUGIN_ICONS` list (19 names, grouped by TASK in comments), merged into the imported names before the missing-name check. `SKIPPED_FILES` is unchanged.
- `packages/desktop-gpui/crates/icons/src/names.rs`: regenerated; 19 added variants, `ALL` entries, `path` arms, and table rows.
- `packages/desktop-gpui/crates/icons/src/lib.rs`: new test `plugin_port_icons_load` only.
- `packages/desktop-gpui/crates/icons/assets/*.svg`: 19 new generated files (`message-circle-question-mark.svg` for `MessageCircleQuestion`).

## Contract

- `bachuc_icons::IconName` gains `Route`, `Columns3`, `CornerLeftUp`, `CornerDownRight`, `Laptop`, `PencilOff`, `Scale`, `ClipboardCheck`, `MessageCircleQuestion`, `Hand`, `LoaderCircle`, `Quote`, `MessagesSquare`, `Power`, `Languages`, `Send`, `FileImage`, `FileCode`, `CloudDownload`.
- Each path is `lucide/<file>.svg` and loads a non-empty SVG through `bachuc_icons::Assets`.
- No existing variant, path, or SVG changed.

## Verified

- `node .../generate-icons.mjs` twice -> first run changed 20 (19 SVGs + names.rs); second run `missing: 0, changed: 0`.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-icons` -> 13 passed, 0 failed.
- `git diff -U0 -- .../names.rs | rg '^-[^-]'` -> no match.
- `git status --short -- .../icons/assets` -> exactly 19 new `.svg` files, none modified.
- `npm run lint -- .../generate-icons.mjs` -> exit 0, 0 warnings, 0 errors.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-icons` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> Finished dev profile (no exhaustive `IconName` match broke).

## Anti-goal

- No existing icon changes: removed-line check on `names.rs` empty and no modified SVG (2026-10-16, local).

## Lessons

- The plugin list lives in the generator, so the namespace import in `plugins/icons.ts` stays skipped.
- Lucide renamed the file for `MessageCircleQuestion` to `message-circle-question-mark`; the name index resolves it.
