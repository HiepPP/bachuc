# Plan Context

## Shared Context

- Goal: close the parity gaps that the previous plan left as follow-ups ([watchtower/archive/20261016-gpui-electron-parity](watchtower/archive/20261016-gpui-electron-parity)): read its [LEARN.md](watchtower/archive/20261016-gpui-electron-parity/LEARN.md), its [QUESTIONS.md](watchtower/archive/20261016-gpui-electron-parity/QUESTIONS.md) (Q-073 to Q-088), and [packages/desktop-gpui/docs/parity-audit.md](packages/desktop-gpui/docs/parity-audit.md).
- The app is pure Rust and GPUI. Electron is only the look to match ([ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md)).
- Branch: `feat/bachuc-gpui`. Builders never commit, push, or switch branches.
- Proof for TASK-157 to TASK-167 was code tests only (owner, 2026-10-16). Builders never open an app window and run code tests only. For TASK-168 to TASK-192, the UI checker also takes background screenshots (owner, 2026-10-16): GPUI and the in-place Electron build `packages/desktop/release/mac-arm64/Bachuc.app` (6770, built 2026-10-15), both opened with `open -g`, never brought to the front, no pointer. A stale or blank frame (Q-069) is `UNVERIFIED`. Where the 2026-10-15 build is older than the Electron source, take the measure from the source. Client `--ignored live_` tests on live `ws://127.0.0.1:6768/ws` are allowed only when live already runs; never start, stop, or restart a daemon, and never touch release (6767).
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
- Second scope (owner, 2026-10-16 drill, TASK-168 to TASK-192): the agent composer matches Electron `packages/app/src/composer` part by part, split into component files that mirror Electron's; the New agent screen becomes the Electron New workspace screen (`packages/app/src/screens/new-workspace-screen.tsx`) with a draft composer; the remaining audit gaps of the explorer, the timeline, and the tab menu stubs; and native Rust ports of every plugin client part per [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md).
- Out of scope (owner): header VS Code and Commit/PR buttons, the sidebar PR line, mic, dictation, and voice, the extra Settings pages, History, Schedules, fork and rewind, extra themes, and the `html-reply` plugin.
- Plugin clients (ADR-0045): each plugin's client is a crate in `packages/desktop-gpui/plugins/<id>`, a workspace member that shares the one gpui-kit build. The server part stays in `hiep-plugins/plugins/<id>`. A plugin shows only when the active host's `plugin.list` reports it installed and enabled. The React client parts are frozen.
- Small plugins (owner): stop-orb becomes a GPUI animation; claude-look ports only its palette, not its web CSS or width change; idle-runtime-closer keeps its pill hidden, as Electron does.
- Defaults (owner): projects stay sorted by name (Q-085); the find focus ring uses `accent`, as Electron does (Q-098); the hard-coded edit-diff row in `crates/timeline` moves into the edit-diffs plugin crate.

## Open Decisions

- None.

## References

- Protocol: [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (agent snapshot `requiresAttention` line 879, `features` line 861, `set_agent_feature_request` line 2049, workspace `diffStat` lines 4158-4164, project `projectKey` line 4313, `ParsedDiffFileSchema` lines 2804-2814, `isGit` lines 3978 and 5363, commits schemas lines 2396-2420 and 5702-5725)
- Electron: [packages/app/src/git/diff-pane.tsx](packages/app/src/git/diff-pane.tsx), [packages/app/src/git/commits-section/commits-section.tsx](packages/app/src/git/commits-section/commits-section.tsx), [packages/app/src/composer/agent-controls/index.tsx](packages/app/src/composer/agent-controls/index.tsx), [packages/app/src/appearance/apply.ts](packages/app/src/appearance/apply.ts), [packages/highlight/src/themes.ts](packages/highlight/src/themes.ts), [packages/app/src/components/provider-icons.ts](packages/app/src/components/provider-icons.ts)
- GPUI client: [packages/desktop-gpui](packages/desktop-gpui)
