# Plan Context

## Shared Context

- Scope is the Rust GPUI client in [packages/desktop-gpui](packages/desktop-gpui). Read Electron source in [packages/app](packages/app) to match it; do not change it.
- The previous plan: [watchtower/archive/20261018-gpui-zoom-and-follow-ups/LEARN.md](watchtower/archive/20261018-gpui-zoom-and-follow-ups/LEARN.md). A layout rule that compares widths must work in 100% units (divide by `bachuc_theme::zoom`).
- Sizes use `bachuc_theme::zoomed`. Run cargo only through `packages/desktop-gpui/cargo.sh` with `CARGO_BUILD_JOBS=4`.
- Put each rule in a pure function and test it in the crate `tests/` folder or the module tests.

## Screen Check Method

- The lead checks screens by computer use (owner request, 2026-10-19). Build the check bundle per the archived `AUTORUN.md` ## Environment, open it with `open -n --env BACHUC_WINDOW=<WxH>`, raise the window with the AX `Raise` action before each screenshot, and navigate with `cmd+k` (MEMORY 2026-10-19).
- Set the zoom in `~/Library/Application Support/BachucGpui/appearance.json` before launch, and restore the owner's 1.25 after.
- Mock data on live: the threads `mock TASK-252 outline` (14 prompts) and `mock TASK-252 long outline` (60 prompts) in `/tmp/wt-mock-244`; make more with `npm run cli -- run --provider mock/ten-second-stream`.

## Decisions

- Owner answers, 2026-10-19: Q-162 shrink labels like Electron; Q-163 cut the placeholder before the hint; Q-168 add the 140 ms ease. The archive ID migration is skipped again.

## Open Decisions

- None.
