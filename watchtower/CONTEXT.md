# Plan Context

## Shared Context

- Goal: close the parity gaps that the previous plan left as follow-ups ([watchtower/archive/20261016-gpui-electron-parity](watchtower/archive/20261016-gpui-electron-parity)): read its [LEARN.md](watchtower/archive/20261016-gpui-electron-parity/LEARN.md), its [QUESTIONS.md](watchtower/archive/20261016-gpui-electron-parity/QUESTIONS.md) (Q-073 to Q-088), and [packages/desktop-gpui/docs/parity-audit.md](packages/desktop-gpui/docs/parity-audit.md).
- The app is pure Rust and GPUI. Electron is only the look to match ([ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md)).
- Branch: `feat/bachuc-gpui`. Builders never commit, push, or switch branches.
- Proof is code tests only (owner, 2026-10-16): unit tests, crate tests, and the client's fake-daemon tests. No screenshot, `cua_repl`, AX read, or `measure.sh` check. A builder must not open any app window. Client `--ignored live_` tests on live `ws://127.0.0.1:6768/ws` are allowed only when live already runs; never start, stop, or restart a daemon, and never touch release (6767).
- Memory limit (owner, 2026-10-16, after an out-of-memory panic): at most 2 builders at once. Every cargo command runs with `CARGO_BUILD_JOBS=4`, one at a time, scoped with `-p <crate>`. Use only `packages/desktop-gpui/cargo.sh`.
- New views use `bachuc_theme` tokens and metrics and `bachuc_icons` icons, never literal colors, sizes, or glyphs. Every new button, row, tab, input, and menu item gets a role and a label; every `track_focus` element gets a role.
- GPUI lessons from the previous plan:
  - Never toggle `display` (hidden or flex) from hover or `group_hover`; it panics with "must call prepaint before paint". Swap by visibility.
  - A client request sent before `ClientEvent::Connected` fails and is not retried. Load after `Connected` and again after each reconnect.
  - gpui block layout collapses adjoining vertical margins; Electron flex columns add them. Use padding or a relative offset where a gap must add.
  - The gpui-kit `Input::h` shadows `Styled::h`; call `Styled::h(input, ..)` for a single-line height.
  - GPUI's default line height is 1.618; Electron text uses about 1.2.
- Hold a strong `Entity<Client>` in each view; a `cx.subscribe` holds its emitter weakly.
- `Cargo.lock` is generated and does not tie groups together; the lead commits it with a group.
- Format Rust with `packages/desktop-gpui/cargo.sh fmt -p <crate>`; format markdown and JS with `npm run format:files -- <file> <file>`.

## Decisions

- Scope (owner, 2026-10-16 drill): client fields Q-074, Q-075, Q-077, Q-078, Q-079, Q-081; explorer multi-file diff, Files toolbar, and Commits footer with commit diffs (Q-082); theme font and syntax API with Settings rows (Q-083); vendored gpui-base (Q-087, [ADR-0044](watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md)); provider logos; reply text drift; tab tooltip, context menu, and maximize; find bar focus ring; diff syntax colors.
- Multi-file diff data: one `checkout.diff.get` request returns every file; reload on `CheckoutChanged`. No diff subscription (owner).
- Font family controls are pickers of the installed font families, not free text (owner; differs from Electron on purpose).
- Kept defaults: Q-085 sidebar sorts projects by name; Q-084 Settings keeps five pages; Q-080 placeholder keeps "@files".

## Open Decisions

- None.

## References

- Protocol: [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (agent snapshot `requiresAttention` line 879, `features` line 861, `set_agent_feature_request` line 2049, workspace `diffStat` lines 4158-4164, project `projectKey` line 4313, `ParsedDiffFileSchema` lines 2804-2814, `isGit` lines 3978 and 5363, commits schemas lines 2396-2420 and 5702-5725)
- Electron: [packages/app/src/git/diff-pane.tsx](packages/app/src/git/diff-pane.tsx), [packages/app/src/git/commits-section/commits-section.tsx](packages/app/src/git/commits-section/commits-section.tsx), [packages/app/src/composer/agent-controls/index.tsx](packages/app/src/composer/agent-controls/index.tsx), [packages/app/src/appearance/apply.ts](packages/app/src/appearance/apply.ts), [packages/highlight/src/themes.ts](packages/highlight/src/themes.ts), [packages/app/src/components/provider-icons.ts](packages/app/src/components/provider-icons.ts)
- GPUI client: [packages/desktop-gpui](packages/desktop-gpui)
