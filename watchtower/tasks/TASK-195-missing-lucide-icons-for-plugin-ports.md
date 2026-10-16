# TASK-195 Missing lucide icons for the plugin ports

Group: U (writes `crates/icons` only)
Class: mechanical

## Brief

Goal: Add the 19 lucide icons that the GPUI plugin ports draw and `bachuc_icons::IconName` lacks, through the existing icon generator.

Change: `bachuc_icons::IconName` holds only the icons that `packages/app/src` imports, so plugin icon names such as `Route` and `Scale` are missing -> the generator also emits a fixed list of plugin icons, and `IconName` gains 19 variants.

Design: the generator [packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs](packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs) (imported names 43-60, the lucide name index 63-73, SVG render 88-101, names table 110-153, run 155-189) and the generated [packages/desktop-gpui/crates/icons/src/names.rs](packages/desktop-gpui/crates/icons/src/names.rs). Plugins name icons by their lucide export name, resolved in Electron by [packages/app/src/plugins/icons.ts](packages/app/src/plugins/icons.ts) (`findPluginIcon`, lines 6-13). Generator conventions: [watchtower/tasks/TASK-158-outcome.md](watchtower/tasks/TASK-158-outcome.md).

Boundaries:

- Add a constant `PLUGIN_ICONS` to `generate-icons.mjs` with exactly these 19 lucide export names, grouped by the TASKs that draw them in comments:
  - `Route` (TASK-187, jev-orchestrator).
  - `Columns3`, `CornerLeftUp`, `CornerDownRight`, `Laptop`, `PencilOff` (TASK-189 and TASK-193, board).
  - `Scale`, `ClipboardCheck`, `MessageCircleQuestion`, `Hand`, `LoaderCircle` (TASK-194, watchtower-board).
  - `Quote`, `MessagesSquare`, `Power`, `Languages`, `Send`, `FileImage`, `FileCode`, `CloudDownload` (TASK-181, TASK-183, TASK-185, and TASK-186).
- Merge the list into the imported names before the lookup, so the existing missing-name check also covers it. All 19 are exports of `node_modules/lucide-react-native` and none is in `IconName` today (checked 2026-10-16). `MessageCircleQuestion` maps to the file `message-circle-question-mark`.
- Keep `SKIPPED_FILES`: the plugin namespace import in `plugins/icons.ts` still must not pull every lucide icon.
- Regenerate with `node packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs`; never hand-edit `src/names.rs` or the SVGs. A second run changes nothing.
- Keep every existing variant, path, and public item of `bachuc-icons`. Adding variants is safe only while no crate matches `IconName` exhaustively without a wildcard; the app build checks that.

Files:

- [packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs](packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs) (the `PLUGIN_ICONS` list and its merge)
- [packages/desktop-gpui/crates/icons/src/names.rs](packages/desktop-gpui/crates/icons/src/names.rs) (generated)
- [packages/desktop-gpui/crates/icons/src/lib.rs](packages/desktop-gpui/crates/icons/src/lib.rs) (only the new test)
- [packages/desktop-gpui/crates/icons/assets](packages/desktop-gpui/crates/icons/assets) (19 new generated SVGs)

Interfaces:

- Produces: `bachuc_icons::IconName` variants `Route`, `Columns3`, `CornerLeftUp`, `CornerDownRight`, `Laptop`, `PencilOff`, `Scale`, `ClipboardCheck`, `MessageCircleQuestion`, `Hand`, `LoaderCircle`, `Quote`, `MessagesSquare`, `Power`, `Languages`, `Send`, `FileImage`, `FileCode`, and `CloudDownload`, each with a `lucide/<file>.svg` path served by `bachuc_icons::Assets`, for TASK-181, TASK-183, TASK-185, TASK-186, TASK-187, TASK-189, TASK-193, and TASK-194

Expected result:

- `IconName::ALL` has 19 more entries, and each new variant's path loads a non-empty SVG through `Assets`.
- No existing variant, path, or SVG changes.

Anti-goal: no existing icon changes; tripwire: `git diff -U0 -- packages/desktop-gpui/crates/icons/src/names.rs packages/desktop-gpui/crates/icons/assets` shows no removed line and no modified existing SVG; read at completion.

## Verify

- `node packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs` run twice -> the second run reports `changed: 0` and `missing: 0`
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-icons` -> pass, including a new test that each of the 19 variants is in `IconName::ALL` and `Assets.load(variant.path())` returns bytes
- `git diff -U0 -- packages/desktop-gpui/crates/icons/src/names.rs | rg '^-[^-]'` -> no match
- `git status --short -- packages/desktop-gpui/crates/icons/assets` -> exactly 19 new `.svg` files and no modified one
- `npm run lint -- packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs` -> exit 0
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-icons` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
