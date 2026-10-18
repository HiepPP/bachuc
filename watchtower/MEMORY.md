# Watchtower Memory

## Purpose

Read this file before creating or implementing Watchtower tasks in this repo.

This file holds long-term intent. It keeps task planning aligned across sessions.

## Core Intent

- This checkout is the user's own Paseo fork, branded Bachuc. The release home is `~/.bachuc`; other internal names stay `paseo` per [ADR-0009](watchtower/decisions/ADR-0009-bachuc-home-over-paseo-home.md). The user's plugins live in `hiep-plugins/plugins/`.
- Two instances exist: release, this repo built and installed as the daily app `/Applications/Bachuc.app` (`PASEO_HOME=~/.bachuc`, port 6767), and live, run from source (port 6768). Stock Paseo from the upstream GitHub is no longer used. Plugins install into release and live.

## Planning Rules

- The fork version must match `X.Y.Z`, `X.Y.Z-beta.N` (N below 999), or `X.Y.Z-hiep`, because `packages/app/native-release-version.js` derives native build numbers from it; `-hiep` was added to that pattern on 2026-10-02. The fork uses `1.0.0-hiep`; plugins require `>=0.10.2-beta.900`, which `1.0.0-hiep` meets through its `1.0.0` core.

- Build the release with `hardenedRuntime=false`; ad-hoc signing with hardened runtime fails at launch.

## Source Anchors

- `packages/plugin/src/client/contracts.ts`: client plugin API.
- `packages/plugin/src/server/lifecycle.ts`: server plugin lifecycle events and before hooks.

## Verification Bias

- `npm run typecheck` and `npm run lint -- <files>` after every change.
- `npx vitest run <file> --bail=1` for changed test files only.

## Avoid

- Avoid editing "Paseo" to "Bachuc" in the locale files while the fork merges upstream. Reason: [ADR-0006](watchtower/decisions/ADR-0006-brand-transform-over-locale-edits.md).
- Avoid the macOS executable name `Paseo` and a top-level `executableName` change while Linux and Windows launchers expect `Paseo`. Reason: [ADR-0007](watchtower/decisions/ADR-0007-bachuc-executable-over-paseo.md).
- Avoid a new desktop bundle ID, such as `vn.bachuc.desktop`, while the owner does not own a Bachuc domain. Reason: [ADR-0008](watchtower/decisions/ADR-0008-github-bundle-id-over-bachuc-domain.md).
- Avoid running sidebar host filters together with an active host while one host is active. Reason: [ADR-0001](watchtower/decisions/ADR-0001-active-host-over-host-filters.md).
- Avoid an offline-only CSP for HTML reply frames while the owner wants agent pages that load CDN libraries and call APIs. Reason: [ADR-0002](watchtower/decisions/ADR-0002-networked-html-replies-over-offline-only.md).
- Avoid renaming internal identifiers (the `paseo` CLI, `PASEO_*` env vars, the `@getpaseo/*` scope, the plugin API, and `paseo://`), a move of the old `~/.paseo` instead of a clone, and a mobile rebrand while the fork merges upstream and plugins depend on these names. Reason: [ADR-0009](watchtower/decisions/ADR-0009-bachuc-home-over-paseo-home.md).
- Avoid upstream hosted services (relay, web app, hub, update feed, EAS) and a self-hosted relay while the owner reaches the host over LAN or Tailscale. Reason: [ADR-0005](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md).
- Avoid required picker fields on every attachment source, a separate `addComposerAttachmentOpener` API, and an `onOpen` on `addComposerAttachment` while chip press handlers must survive a reload and work on other clients. Reason: [ADR-0003](watchtower/decisions/ADR-0003-chip-only-attachment-sources.md).
- Avoid TASK rows, plan grids, a Gantt as Board's main view, a plan line, or a Board Needs you column while the Watchtower page shows plan depth one click away; keep Board on triage and show a thread's TASK as a chip. Reason: [ADR-0014](watchtower/decisions/ADR-0014-task-chips-only-over-plan-state-on-board.md).
- Avoid plan questions, manual checks, or their counts on Board, and answer targets picked without showing them, while `QUESTIONS.md` does not record the asking thread. Reason: [ADR-0015](watchtower/decisions/ADR-0015-questions-on-the-watchtower-page-over-board.md).
- Avoid moving a feature into a plugin, or keeping a host change thin, only to save upstream merge cost, while the fork has moved too far from upstream for that cost to matter. Reason: [ADR-0017](watchtower/decisions/ADR-0017-native-host-features-over-thin-host-changes.md).
- Avoid a plugin group renderer API and a separate turn-timeline plugin for the agent turn step list while the host owns tool call grouping and edit-diffs owns the diff. Reason: [ADR-0016](watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md).
- Avoid treating only the last assistant message group as a turn's answer while providers can split one answer into several messages. Reason: [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md).
- Avoid a fold, chevron, or Collapse button on the turn step list, and the old Overview badge on desktop, while the owner reads turns without a click per turn. Reason: [ADR-0019](watchtower/decisions/ADR-0019-always-open-step-list-over-folding-group.md).
- Avoid palette-only label colors and scheme-blind fixed hues in the turn step list while the owner keeps the design's Explore, Edit, and Verify hues. Reason: [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md).
- Avoid branching on `theme.colorScheme` in a `StyleSheet.create` factory, `withUnistyles` label wrappers, and one fixed tone for both schemes while Unistyles hands the web style factory a theme of CSS variables. Reason: [ADR-0022](watchtower/decisions/ADR-0022-scheme-label-tones-in-the-theme-over-color-scheme-branches.md).
- Avoid steps made of the tool calls before a note, and a per-provider rule, while agents in use write their note before they act. Reason: [ADR-0021](watchtower/decisions/ADR-0021-note-opens-its-step-over-tools-before-the-note.md).
- Avoid restarting TASK or Q-IDs at 001 in a new plan, and avoid per-plan or per-kind ID prefixes, while one repo has one `watchtower/` folder. Reason: [ADR-0011](watchtower/decisions/ADR-0011-repo-wide-ids-over-per-plan-ids.md).
- Avoid a pill wheel with a step cooldown, a wheel that steps a pill with its list closed, wraps at the ends, or changes the provider from the model pill, and avoid wheel handling on the sheet surface, while the owner wants hover to open a pill's list, one notch per step, and the list closed when the pointer leaves the pill. Reason: [ADR-0025](watchtower/decisions/ADR-0025-wheel-opens-the-list-and-steps-over-closed-pill-step.md).
- Avoid a three.js Fleet map tab in the app, and a hidden tab that keeps `three` in the bundle, while `three` loads at app start and the agent tree lives as 2D UI on the Watchtower page. Reason: [ADR-0028](watchtower/decisions/ADR-0028-no-fleet-map-tab-over-three-js-fleet-map.md).
- Avoid a branch timeline, Running now, or Up next on the Watchtower page, and a second tab or a side-by-side timeline for the plan tree, while the owner chose the tree after comparing mockups. Reason: [ADR-0029](watchtower/decisions/ADR-0029-plan-tree-over-branch-timeline.md).
- Avoid `glm-acp-agent` as the main GLM path, a config-entry-only GLM setup, and a GLM provider that shares `~/.claude`, while the Claude path passed the live probe and the owner's hooks and plugins live in `~/.claude`. Reason: [ADR-0030](watchtower/decisions/ADR-0030-built-in-glm-on-claude-client-over-acp-and-config.md).
- Avoid memory-only timelines, unbounded in-memory timelines for live agents, JSONL timeline files, better-sqlite3, Drizzle, and a database for agent records, while `node:sqlite` with FTS5 runs in the Electron daemon runtime and agent listing stays fast in memory. Reason: [ADR-0032](watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md).
- Avoid pixel widths computed from `onLayout` for cells that must share a row on Board, while react-native-web reports a rounded `offsetWidth`. Reason: [ADR-0033](watchtower/decisions/ADR-0033-percent-cells-over-measured-pixel-cells-on-board.md).
- Avoid a Swift or SwiftUI desktop client, and new long-term work on the Electron renderer UI, while the GPUI client measured 6.5% CPU against 24% for Swift and 1.2 GB or more for Electron on the same stream. Reason: [ADR-0034](watchtower/decisions/ADR-0034-rust-gpui-client-over-swift-and-electron.md).
- Avoid hand-written Rust protocol structs while the zod schemas can export JSON Schema for code generation. Reason: [ADR-0035](watchtower/decisions/ADR-0035-generated-protocol-types-over-hand-written-rust-types.md).
- Avoid a live WebView inside each timeline row, and SVG export of diagrams, while `gpui-wry` views do not clip inside GPUI scroll lists and GPUI's usvg drops `foreignObject` labels and KaTeX HTML. Reason: [ADR-0037](watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md).
- Avoid WebView panes, a hidden WebView or embedded JS engine for plugin clients, declarative plugin widgets, and moving plugin client logic into server parts, while the owner wants native Rust plugin clients and freezes Electron. Reason: [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md).
- Avoid a higher GPUI CPU budget, a slower timeline redraw while a turn streams, and folding or capping old steps, while incremental sync keeps a 15-minute turn under 10%. Reason: [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md).
- Avoid a merged "All machines" view, `wss`, the relay, and built-in SSH in the GPUI client while the owner reaches hosts through a tunnel and needs one host on screen. Reason: [ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md).
- Avoid daemon start, stop, updates, CLI install, and deep links in the GPUI client while Electron and the CLI own them. Reason: [ADR-0041](watchtower/decisions/ADR-0041-gpui-client-connects-only-over-daemon-management.md).
- Avoid terminal restore mode `"full-snapshot"` (`0x05` ANSI bytes) in the GPUI client while the daemon sends the `0x04` JSON grid without a restore mode. Reason: [ADR-0042](watchtower/decisions/ADR-0042-json-grid-terminal-restore-over-ansi-snapshot.md).
- Avoid gpui-kit default themes and the gpui-kit `IconName` set in the GPUI client, and porting every dark theme variant now, while the owner wants the exact Electron look in `light` and `claude` dark. Reason: [ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md).
- Avoid generic lucide icons in the GPUI Open-in menu while the owner wants each app to show its own icon. Reason: [ADR-0046](watchtower/decisions/ADR-0046-real-app-icons-in-open-in-menu-over-lucide.md).

## Learnings

### 2026-10-01 - Plugin host APIs

- Learned: the app runs React Compiler. A `useMemo` input that the body reads only through `void` is dropped from the compiled cache keys, so state bumped by an external store never re-runs the memo. Use `"use no memo"` or read the value for real.
- Source: `packages/app/src/plugins/sidebar/index.tsx`
- Use next time: check the compiled bundle (`app-dist/_expo/static/js/web/index-*.js`) when a hook ignores an update.
- Learned: text a plugin appends in `before("agent.prompt")` must be wrapped in `<paseo-plugin-context>`, because providers replay their own history after a daemon restart.
- Source: `packages/server/src/server/agent/plugin-prompt-context.ts`
- Use next time: verify plugin prompt changes after a daemon restart, not only live.

### 2026-10-06 - Autorun 20261006-t3code-orchestration-port

- Learned: the lefthook pre-commit hook format-checks every staged `*.md`, including `watchtower/` plan files. The formatter reads a bare `**` glob as emphasis and rewrites it.
- Source: `lefthook.yml`; the ADR-0002 Scope line.
- Use next time: run `npm run format:files` on plan files before staging, and put globs in code spans.
- Learned: finish notices now wait `FINISH_NOTICE_WINDOW_MS` (1500 ms), but `vi.waitFor` gives up after 1000 ms by default. A fake timer made before `vi.useRealTimers()` never fires.
- Source: `packages/server/src/server/agent/agent-prompt.ts`; `packages/server/src/server/agent/mcp-server.test.ts`.
- Use next time: give finish-notice waits a longer timeout, and keep fake timers on through `vi.waitFor`.
- Learned: `session.test.ts` mocks count `getAgent` calls, so an extra lookup in a lifecycle command fails it even when the code is correct.
- Source: `packages/server/src/server/session.test.ts` ("cancel_agent_request reports refusal only through its response"); TASK-027 outcome.
- Use next time: run `session.test.ts` after any change to `packages/server/src/server/agent/lifecycle-command.ts`.

### 2026-10-07 - Autorun 20261006-t3code-orchestration-port

- Learned: after a `packages/plugin` type change, the app typecheck reads stale declarations until the SDK is rebuilt.
- Source: TASK-030 and TASK-034 outcomes.
- Use next time: run `npm run build:plugin` before `npm run typecheck`.
- Learned: app tests that load Expo fail from the repo root with `__DEV__ is not defined`.
- Source: TASK-030 and TASK-034 runs.
- Use next time: run app vitest files from `packages/app`, with `--project browser` for `*.browser.test.ts`.
- Learned: the agent stream view mounts before its authoritative history arrives, and the vitest Lucide stub lacks many icons, such as `Quote`.
- Source: `packages/app/src/panels/agent-panel.tsx`; `packages/app/test-stubs/lucide-react-native.ts`; TASK-034 outcome.
- Use next time: wait for `isAuthoritativeHistoryReady` before checking loaded items, and use `Blocks` as the icon in plugin tests.
- Learned: in vitest browser tests, app sources compile with the classic JSX runtime, and the Unistyles stub ignores `withUnistyles` mappings.
- Source: `packages/app/src/components/question-form-card.browser.test.tsx`; `packages/app/test-stubs/react-native-unistyles.ts`; TASK-032 outcome.
- Use next time: call `vi.stubGlobal("React", React)` before rendering app components, and test theme mappings as pure functions.

### 2026-10-07 - Autorun 20261007-watchtower-process-overview

- Learned: a plugin `lint` script without `--disable-nested-config` fails before it reads any file. oxlint treats the repo root `.oxlintrc.json` as a nested config and rejects its `options.typeAware` key.
- Source: `hiep-plugins/plugins/watchtower-board/package.json`; `hiep-plugins/plugins/pr-watch/package.json`; TASK-047 outcome.
- Use next time: give every plugin `lint` script the `--disable-nested-config` flag.

### 2026-10-08 - Autorun 20261007-bachuc-rebrand

- Learned: electron-builder names the macOS bundle after `executableName`, and names `CFBundleName` and the helper apps after `productName`. The packaging hooks and the nix build expect `Paseo.app`.
- Source: `node_modules/app-builder-lib/out/appInfo.js`; `packages/desktop/scripts/after-pack.js`; TASK-037 outcome.
- Use next time: keep `executableName: Paseo`, and check that `IS_RELEASE_APP` cannot match the build output path.
- Learned: the desktop daemon starts through `<name> Helper.app`, found by the executable name, `app.name`, or the one helper in Frameworks. The release script builds with `-c.productName=Bachuc`.
- Source: `packages/desktop/src/daemon/runtime-paths.ts`; `scripts/paseo-release.sh`.
- Use next time: when an app name changes, check that the daemon still finds its helper app.
- Learned: `magick` on this machine has no SVG delegate, because `rsvg-convert` is missing, and it writes `.ico` entries as BMP only. `sharp` in `node_modules` bundles librsvg.
- Source: `magick -list delegate`; TASK-038 outcome.
- Use next time: render SVG to PNG with `sharp`, give the SVG root a `width` and `height` instead of a density, and pack PNG files into `.ico` with a short script.
- Learned: CLI tests import `@getpaseo/server` from its `dist`, so after a server change they test the old code and can pass falsely.
- Source: `packages/server/package.json` exports; `packages/cli/src/commands/daemon/pair.test.ts`; TASK-039 outcome.
- Use next time: run `npm run build:server` before CLI tests that touch server code.
- Learned: desktop updates are off through two constants, `APP_UPDATES_ENABLED` in the desktop package and `DESKTOP_APP_UPDATES_ENABLED` in the app. An upstream merge that restores the `publish` block in `electron-builder.yml` does not turn updates on again.
- Source: `packages/desktop/src/features/app-updates-enabled.ts`; `packages/app/src/desktop/updates/desktop-updates.ts`; TASK-040 outcome.
- Use next time: after an upstream merge that touches the updater or the About rows, check that both constants are still `false` and still read by `isPackaged` and `shouldShowDesktopUpdateSection`.
- Learned: the old name `Paseo Fork` stays on purpose in `OLD_APP` (the Trash move of the old app) and `SIGN_IDENTITY` (the `Paseo Fork Local` certificate). The desktop log folder follows `app.name`, so release logs go to `~/Library/Logs/Bachuc/`.
- Source: `scripts/paseo-release.sh`; `packages/desktop/src/main.ts`; TASK-037 and TASK-042 outcomes.
- Use next time: grep for `Paseo Fork\.app` to find stale install paths, not for `Paseo Fork`.

### 2026-10-08 - Autorun 20261007-bachuc-rebrand, TASK-043

- Learned: TASK-043 replaced the earlier advice to keep `executableName: Paseo`. The macOS build is now `Bachuc.app` with `Contents/MacOS/Bachuc`, through `mac.executableName`. The top-level `executableName: Paseo` stays, because electron-builder also names the Linux binary and the Windows `.exe` after it.
- Source: `packages/desktop/electron-builder.yml`; `node_modules/app-builder-lib/out/appInfo.js`; TASK-043 outcome.
- Use next time: `IS_RELEASE_APP` in `packages/desktop/src/main.ts` must check the install path `/Applications/Bachuc.app/`, because the build output has the same bundle name. The macOS consumers of the executable name use `Bachuc`; the Linux and Windows consumers use `Paseo`.

### 2026-10-08 - Autorun 20261007-bachuc-rebrand, TASK-044

- Learned: the fork hides the sidebar footer Hosts, Import session, and Help buttons with `SHOW_FOOTER_EXTRAS = false` since 2026-09-30. An e2e spec that clicks `sidebar-help`, `sidebar-import-session`, or the host picker fails before its own checks.
- Source: `packages/app/src/components/left-sidebar.tsx`; commit `5e8568f6d`; TASK-044 outcome.
- Use next time: before you fix a sidebar e2e failure, check that the control renders. The help menu tests in `packages/app/e2e/browser/sidebar-help.spec.ts` are `test.fixme` until the button shows (Q-019).

### 2026-10-12 - Plan 20261012-repo-wide-ids-and-board-lanes

- Learned: the app typechecks against `@getpaseo/plugin` in `dist`, so a client contract change needs `npm run build --workspace=@getpaseo/plugin` first. Plugins in `hiep-plugins/plugins/*` link the SDK to `packages/plugin` and see the change at once.
- Source: TASK-062 outcome.
- Use next time: rebuild the plugin package before you read an app type error after a contract edit.
- Learned: watchtower-board's `tests/overview.test.ts` reads this repo's real `watchtower/` files, so an ADR status change or archive edit can fail it.
- Source: `hiep-plugins/plugins/watchtower-board/tests/overview.test.ts`.
- Use next time: run that test after you change `watchtower/DECISIONS.md` or an archived plan.

### 2026-10-12 - Plan 20261012-turn-step-list

- Learned: app view tests run in jsdom with the real react-native-web: `testID` becomes `data-testid`, `element.click()` fires `onPress`, and Unistyles styles render inline. RN-web 0.21 ignores `accessibilityState`, so use `aria-pressed` or a `dataSet` flag to assert a pressed state.
- Source: `packages/app/src/tool-calls/detail-level/turn/view.test.tsx`; TASK-073 outcome.
- Use next time: test a new app view as a plain `*.test.tsx` before you reach for a browser test.
- Learned: `TranslationResources` derives from `en`, so a new key in `en.ts` must also land in all 8 other locale files under `packages/app/src/i18n/resources/` for typecheck to pass.
- Source: TASK-073 outcome.
- Use next time: plan locale edits for every resource file when a TASK adds UI strings.
- Learned: the root `npm run format:files` and `format:check:files` skip `hiep-plugins/` and exit 2 with "Expected at least one target file" when only plugin files are passed.
- Source: TASK-074 outcome.
- Use next time: format plugin files with the plugin's own `npm run format`, and check them with `npx oxfmt --check` from the plugin folder.

### 2026-10-13 - Plan 20261013-turn-step-list-follow-ups

- Learned: green app tests do not prove a theme-dependent style works. The vitest Unistyles stub passes a plain theme, but web passes CSS variables, so a value computed from a theme string crashed live at start.
- Source: ADR-0022; TASK-094 outcome.
- Use next time: after a `StyleSheet.create` change that computes from theme values, reload live and read the window before you call the TASK done.

### 2026-10-13 - Plan 20261013-composer-pill-wheel-select

- Learned: a composer pill ref is shared by the popover anchor, `TooltipTrigger`, and now the wheel hook. `composeViewRefs` returns a new callback per call, and `TooltipTrigger` passes a new callback ref each render, so an unmemoized composed ref detaches and reattaches on every commit.
- Source: TASK-079 and TASK-080 outcomes; TASK-080 review.
- Use next time: wrap `composeViewRefs` in `useMemo`, and keep the hook's node in state so the extra commit stays a no-op render.
- Learned: two plan specs named the wrong render path. The desktop provider pill is a raw `ComboboxTrigger`, not an `AgentControlTrigger`, and the composer model pill takes the `ComboboxTrigger` branch of `CombinedModelSelector` with a `toolbar` prop, not `renderTrigger`.
- Source: TASK-080 and TASK-081 outcomes; Q-030, Q-033.
- Use next time: when a spec wires a shared component, list each caller and the branch it renders before you name the path.

### 2026-10-13 - Plan 20261013-fleet-map

- Learned: the label fallback functions in `workspace-screen.tsx` and `workspace-desktop-tabs-row.tsx` are not exhaustive over `WorkspaceTabTarget`. A new tab kind falls through to the Agent label with no type error.
- Source: TASK-088 outcome.
- Use next time: when a spec adds a tab kind, name both label functions in `Files:` and test the label by hand.
- Learned: in zsh, a variable that holds several paths is not split into words, so `npm run lint -- $FILES` passes one bad path.
- Source: TASK-086 and TASK-088 outcomes.
- Use next time: pass each path as its own argument or use an array.
- Learned: `cua_repl` cannot move the pointer without a click, so it cannot test hover. A headless browser (`agent-browser`) connected to the live daemon at `127.0.0.1:6768` can, with real mouse moves. Counting `CanvasRenderingContext2D.drawImage` calls probes the frame work of a canvas view.
- Source: TASK-090 outcome; [watchtower/archive/20261013-fleet-map/LEARN.md](watchtower/archive/20261013-fleet-map/LEARN.md).
- Use next time: set the host to port 6768 in the connect form, because its default is 6767 (release).
- Learned: Metro puts a module shared by two async chunks into `__common`, which `index.html` loads at page start. An `import()` split does not make the library lazy at start.
- Source: TASK-090 outcome (`expo export`, 2026-10-13).

### 2026-10-14 - Plan 20261014-watchtower-plan-tree

- Learned: in agent shells, `npm run lint` in a plugin folder can exit 2 with "ESLint output (JSON parse failed)", because an rtk ESLint wrapper intercepts it. The oxlint run itself is clean.
- Source: TASK-109, TASK-110, and TASK-111 outcomes; lead re-check.
- Use next time: run `rtk proxy npm run lint` and trust its exit code.
- Learned: the plugin agent snapshot has no `parentAgentId` or `lastActivityAt`; the parent comes from the label `paseo.parent-agent-id`, and activity from `updatedAt`. An agent that waits for a permission still reports `running`.
- Source: TASK-109 outcome.
- Use next time: read `attentionReason` before `status` when you show who waits for the owner.
- Use next time: read the built output before a plan promises lazy loading.

### 2026-10-14 - Plan 20261014-native-glm-provider

- Learned: a built-in provider id must be added in three lists: `AGENT_PROVIDER_DEFINITIONS`, the `BUILTIN_PROVIDER_IDS` of `packages/protocol/src/provider-config.ts`, and the one in `packages/server/src/server/persisted-config.ts`. The app icon needs `packages/app/src/components/provider-icon-name.ts` too.
- Source: TASK-104 and TASK-106 outcomes.
- Use next time: list all four files when a plan adds a built-in provider.
- Learned: the no-live-restart repo rule leaves every server change unverified on live during implement.
- Source: TASK-104, TASK-105, and TASK-106 outcomes.
- Use next time: plan a live check as a separate owner step after implement.

### 2026-10-15 - Board subagent rows flashing

- Learned: react-native-web gives `onLayout` a width from `offsetWidth`, rounded to a whole pixel. Pixel cells sized from it can overflow a row by less than 1 px and wrap. The wrap changes the height, the scroll bar toggles, and the layout flips without end.
- Source: Board flashing fix on 2026-10-15, ADR-0033.
- Use next time: on web, size cells that share a row with percent or flex values. Use a measured width only for a breakpoint choice.

### 2026-10-15 - Plan 20261015-sqlite-ram-cpu-cuts

- Learned: Vitest resolves `./row-store` in the replica cache to `row-store.native.ts`, and `host-runtime.test.ts` has its own fake `ReplicaRowStore`.
- Source: TASK-126 outcome.
- Use next time: import runtime values from `row-store-schema.ts`, and list `host-runtime.test.ts` in `Files:` when the row store interface changes.
- Learned: FTS5 `unicode61 remove_diacritics 2` does not fold `đ`, and it matches only whole words or word prefixes.
- Source: TASK-128 outcome; Q-054.
- Use next time: fold `đ` to `d` on both sides, and do not promise a mid-word match from the cache path.
- Learned: a coalesced write whose promise waits for the file commit makes every awaited persist slower, and it hangs fake-timer tests that await a persist before they advance the clock.
- Source: TASK-131 outcome; Q-049.
- Use next time: when a spec debounces a write, say whether callers await the commit or only the cache update, and list the tests that await it.
- Learned: the formatter ignores `docs/`, so `npm run format:check:files -- docs/<file>` exits 2 with "Expected at least one target file".
- Source: TASK-133 outcome.
- Use next time: do not put a format check for `docs/` files in a Verify line.
- Learned: many source rows can project to few items: a 3,701-row mock thread gave 88 items, which fit in the first app page, so a UI scroll sent no older-page request. Page it through `fetch_agent_timeline` with the CLI client (`connectToDaemon` from `packages/cli/src/utils/client.ts`) to test SQL pages on live. The mock `five-minute-stream` model gives about 3,700 source rows.
- Source: Q-052 check on 2026-10-15.
- Use next time: plan a live paging check as an RPC script, not a UI scroll.

### 2026-10-15 - Plan 20261015-gpui-desktop-client

- Learned: `typify` 0.8 silently merges JSON Schema types that share a name, so `packages/desktop-gpui/crates/protocol/build.rs` gives each type a unique title and fails the build when typify names a type itself. Generated names come from message tags, for example `AgentStreamPayloadEvent` and `TimelineItem`; read them in `target/debug/build/bachuc-protocol-*/out/protocol.rs`.
- Source: TASK-135 outcome.
- Use next time: name generated Rust protocol types in specs only after you read the generated file.
- Learned: `git diff --exit-code` on a new untracked file always passes. Compare `shasum` of two runs instead.
- Source: TASK-135 outcome.
- Use next time: do not use `git diff` as an idempotence check for a file the TASK creates.
- Learned: GPUI holds a `cx.subscribe` emitter weakly. Dropping the last `Entity<Client>` stops the `bachuc-client` socket thread at once.
- Source: TASK-136 outcome.
- Use next time: keep a strong `Entity<Client>` in the view or app state that owns it.
- Learned: GPUI stops drawing while its window is covered, so `cua_repl` screenshots show stale frames, scroll events land only on a raised window, and `measure.sh` reads come out low. A 60 fps GPUI scroll costs about 9.6% CPU with no content.
- Source: TASK-137 outcome; Q-058.
- Use next time: raise the window before each screenshot or read, and set scroll CPU limits against a no-content control run.
- Learned: one `cua_repl` `getApp` call right after a bundle launch hung for 54 minutes and held the shared screen lock for 3 hours. A raise alone often left a GPUI window unpainted; a raise plus a title-bar click painted it.
- Source: TASK-139 and TASK-138 outcomes.
- Use next time: wrap each `cua_repl` step in `Promise.race` with a timeout, wait a few seconds after launch, and click the title bar after a raise.
- Learned: a GPUI list item that holds a whole 50-step turn cost about 74% CPU on the mock stream; one item per header and per step cost about 4%.
- Source: TASK-138 outcome.
- Use next time: keep GPUI list items small, one per visual row group.
- Learned: the gpui-kit `test-support` dev feature and the tree-sitter `css`, `html`, `toml`, and `yaml` features break workspace resolution on `cocoa` (=0.26.0 against the locked 0.26.1).
- Source: TASK-139 and TASK-138 outcomes; Q-063.
- Use next time: test GPUI view crates through plain functions, and do not enable those features.
- Learned: repeated title-bar clicks count as double-clicks and zoom a GPUI window, which then draws a clipped area. This replaces the title-bar click advice above. Running `open <bundle>` in a loop kept the window in front during a read.
- Source: TASK-141 outcome.
- Use next time: keep a GPUI bundle in front with an `open` loop, and read `getAXState` twice.
- Learned: GPUI cached views (`Entity::cached`) save about 2 CPU points but drop out of the accessibility tree, and a `Role::ComboBox` div did not run its click handler through `cua_repl`.
- Source: TASK-141 outcome.
- Use next time: keep labeled chrome views uncached, and give click targets `Role::Button`.
- Learned: in zsh, `F="a.md b.md"; npm run format:files -- $F` passes one argument, so oxfmt prints "Expected at least one target file" and formats nothing. Two builders reported a passing format check this way, and the lead found 3 unformatted files.
- Source: TASK-142 and TASK-143 lead re-check, 2026-10-16.
- Use next time: pass file lists as separate arguments or with `${=F}`, and treat "Expected at least one target file" as a failed check.
- Learned: Paseo `create_agent` on release failed with "Too many orchestrator sessions; reload the plugin". The `jev-orchestrator` bridge keeps bound leases across reloads (`hiep-plugins/plugins/jev-orchestrator/server/bridge.ts`), so a reload did not help. The run used native builder subagents instead.
- Source: implement run of 2026-10-15.
- Use next time: check that `create_agent` works before a run that needs Paseo builders.
- Learned: `bachuc-timeline` sync was O(entries) per 50 ms flush (full regroup, full hash, header fingerprint over every step), so app CPU grew with the turn: 11.3% at 15 minutes. `grouping::TurnGrouper` regroups only changed turns and gave 8.9%. A GPUI window can also get stuck drawing only part of its area (Q-069).
- Source: TASK-144 outcome.
- Use next time: keep per-flush work in proportion to the change, and check a screenshot for a cut window before each CPU read.

### 2026-10-16 - Implement run memory limit

- Learned: 6 Claude Code builders plus a UI checker, each with cargo builds on one shared `target`, used all 24 GB of RAM and the swap. macOS then hit a watchdog panic and rebooted. The reboot also cleared the builder rules files in `/tmp`. GitNexus `detect-changes` peaked at 730 MB, so it was not the main load.
- Source: owner crash report and the `gpui-electron-parity` run.
- Use next time: on this machine, run at most 2 builders with `CARGO_BUILD_JOBS=4`. Keep run files that must survive a reboot outside `/tmp`.

### 2026-10-16 - GPUI hover display panic

- Learned: an element that toggles `display` (hidden or flex) from hover or `group_hover` panics with "must call prepaint before paint". Prepaint tests hover against the previous frame and skips a hidden child, but paint tests fresh hover and paints that child. GPUI also logs "has no accessibility node" for a focused element that has an id but no role.
- Source: TASK-149 and TASK-155 outcomes.
- Use next time: swap hover content by visibility or stacked layers, never by display. Give every `track_focus` element a role and a label.

### 2026-10-16 - Plan 20261016-gpui-parity-follow-ups

- Learned: GPUI view crates have no headless test context here, so builders put each rule (fallback, clamp, in-flight guard, redraw check) in a pure function and test that. Code that needs `App` or `Context` is checked only by reading it.
- Source: TASK-159, TASK-160, TASK-161, and TASK-163 outcomes.
- Use next time: write GPUI Verify lines against pure model functions, and plan any window check as a separate owner step.
- Learned: the lefthook format hook checks staged `*.md` and `*.json` everywhere, so the vendored `packages/desktop-gpui/vendor/gpui-base` needed an `ignorePatterns` entry in `.oxfmtrc.json`.
- Source: TASK-166 commit, 2026-10-16.
- Use next time: add a vendored folder to `.oxfmtrc.json` before its first commit.

### 2026-10-17 - Plan 20261016-gpui-parity-follow-ups, plugin ports

- Learned: in a file with `use gpui_kit::*`, a test module with `use super::*` picks up GPUI's test macro, which shadows `#[test]` and hits the recursion limit.
- Source: TASK-181 and TASK-184 outcomes.
- Use next time: in those files, import test helpers by name instead of `use super::*`.
- Learned: a Verify grep for forbidden crate names in a plugin `Cargo.toml` also matches the crate's own `bachuc-plugin-<id>` name, so three builders marked it UNSATISFIABLE.
- Source: TASK-189, TASK-190, and TASK-194 outcomes.
- Use next time: scope that grep to the `[dependencies]` section.
- Learned: in a debug build, a small `HashMap` costs about 18 us per build-and-lookup pass; a sorted `Vec` costs 3 us. One 40-task overview derive went from 643 ms to 153 ms per 1,000 runs after the switch.
- Source: TASK-194 outcome.
- Use next time: prefer sorted vectors for small per-frame or per-derive maps in GPUI crates.
- Learned: each crate tested alone with `-p` gets its own gpui-kit build, because cargo unifies features per package selection; `target/debug` held 6 gpui-kit variants.
- Source: TASK-170 and TASK-181 outcomes.
- Use next time: watch free disk during a long run, and clean the dev profile only when no builder runs.

### 2026-10-17 - GPUI timeline scroll lag

- Learned: the GPUI timeline scrolled jerky and slow on long release threads. `bachuc_theme::mono_font_family` called `cx.text_system().all_font_names()` on every call, and that builds a CoreText collection of every installed family (`gpui-pre-apple-0.3.9/src/text_system.rs:108`). The timeline calls it on every draw of a tool row and a code row (`crates/timeline/src/tool_view.rs`, `crates/timeline/src/lib.rs`), and the plugin crates call it per row too, so each scroll frame enumerated the fonts many times.
- Fix: `mono_font_family` picks the family once per process in a `OnceLock` (commit `c1887709e`); the pure `pick_mono_family` holds the rule and has a test. The owner confirmed the scroll is smooth after the fix (2026-10-17).
- Source: owner report and hand test, 2026-10-17; `packages/desktop-gpui/crates/theme/src/lib.rs`.
- Use next time: never call `all_font_names`, or anything that queries the system, from a render or row path; compute it once and cache it. `installed_font_families` still enumerates on each call, so only a picker may call it, never a row.
- Learned: `sample <pid>` taken while the owner scrolled showed the GPUI main thread idle four times, and `cua_repl` `scroll` moved neither the timeline nor the sidebar, so neither proves a GPUI scroll bug or its absence.
- Source: profiles `/tmp/gpui-scroll-sample*.txt` and the cua scroll control test, 2026-10-17.
- Use next time: for a GPUI scroll or lag report, read the per-row render path for system queries first, and confirm with the owner's own scroll on an A/B pair of bundles (for example a `Bachuc GPUI Before.app` built from the base commit).

### 2026-10-17 - How the 15 plugin clients were ported to native GPUI

- Learned: porting the React plugin clients of `hiep-plugins` to Rust GPUI crates (TASK-168 to TASK-195, [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md)) gave UI and behavior the owner called identical to Electron, with about 45,800 Rust lines and no JS in the client. The method that worked:
  - Keep the server part. Each plugin's `index.server.ts` stays in the daemon; the Rust client calls the same plugin RPCs (`plugin.rpc.invoke`, `settings.<id>.read/write/reset`), so data and rules match Electron by construction. Only the client is rewritten.
  - Build the foundation first and freeze its names. The client channel (TASK-168: list, RPC, settings, events) and one host crate, `bachuc-plugin-host` (TASK-170), cover every client API the plugins use: one typed contribution kind per `add*` call, a `PluginContext` for the host capabilities, a `ui` kit for settings rows, and gating by the host's `plugin.list`. Every later spec copied these `Produces` lines word for word.
  - Inventory before specs. One read-only agent listed each plugin's contributions, client state, RPC names and shapes, host APIs, and line counts; one more mapped the host renderers in `packages/app/src/plugins`. Specs then cited Electron `path:line` for every measure, label, icon, and rule.
  - One crate per plugin in `packages/desktop-gpui/plugins/<id>`, a workspace member on the one shared `gpui-kit`. A plugin crate depends only on `bachuc-plugin-host`, `bachuc-client`, `bachuc-theme`, and `bachuc-icons`, never on a view crate or another plugin, so builds run in parallel and no cycle forms. Each exposes only `pub fn plugin()`; the app registers all of them last (TASK-191).
  - Host views draw the contributions natively, one host TASK per surface: composer slots, sidebar, Command Center, settings, timeline transformers and renderers, themes (TASK-171 to TASK-180).
  - Port JS semantics exactly: UTF-16 lengths, the JS trim set, `split(/\r?\n/)`, one-pass replace, regex `$` handling. Pin them with goldens produced by Node from the plugin's own code.
  - Put every rule in a pure function with a test, because GPUI views have no headless test context; read-check the view code; then run screen checks with a UI checker.
  - Split any plugin over about 3,000 client lines into two stages with a crate-private store as the seam (board, watchtower-board).
  - Run a Builder Check of every spec against the code before building; it found missing Deps, wrong line refs, and items no producer gave, which the lead fixed in the specs.
- Source: TASK-168 to TASK-195 specs and outcomes; owner review, 2026-10-17.
- Use next time: for a new plugin, write its server part in `hiep-plugins/plugins/<id>`, then add a GPUI client crate in `packages/desktop-gpui/plugins/<id>` that uses only `bachuc_plugin_host` kinds and calls the server through RPC. If it needs a contribution kind the host lacks, add the kind to `bachuc-plugin-host` first, and draw it in the owning host view. Avoid in plugin and view code: reading a `ListState` inside a row callback, system queries such as `all_font_names` in a render path, and small `HashMap`s on per-frame paths; give every pressable element an explicit `AccessibleAction::Click` handler.

### 2026-10-18 - Implement 20261017-gpui-daily-gaps-and-fixes

- Learned: a GPUI flex child with a negative main-axis margin inside a content-sized container (flex_none or basis auto) makes taffy 0.13 measure the container at 0 width, so its children draw outside the parent and get clipped.
- Source: TASK-209 fix round, `packages/desktop-gpui/plugins/board/src/card.rs` (`actions_offset`).
- Use next time: move content with `relative()` and `left`/`right`, never with a negative margin.
- Learned: an agent-update subscription without `includeArchived` gets `remove`, not an archived upsert, when an agent is archived. A client test must send what the daemon sends for that filter.
- Source: TASK-200 fix round; `packages/server/src/server/agent-updates-service.ts`, `Client::fetch_agent`.
- Use next time: fetch an unknown agent before deciding it does not exist.
- Learned: an AX press runs only an explicit `AccessibleAction::Click` listener reliably; the fallback click lands at the node centre and misses a clipped or covered node. `accesskit_macos` 0.26.3 maps no show-menu action, so a right-click-only menu cannot be opened by a UI checker.
- Source: UI checker runs 2 and 4; TASK-198, TASK-209, TASK-213 outcomes.
- Use next time: give every control and context menu a keyboard or AX Click path when a background UI check must reach it.
- Learned: `BACHUC_NO_ACTIVATE=1` keeps the GPUI app in the background for UI checks (`open -n -g --env BACHUC_NO_ACTIVATE=1 <bundle>`), and the AX "zoom the window" action forces a frame of a background window.
- Source: `packages/desktop-gpui/crates/app/src/main.rs` (`activates_on_launch`); UI checker runs 2 to 4.
- Use next time: build the check app under its own bundle name so it never replaces the owner's running GPUI app.
- Learned: GPUI calls event subscribers in subscription order, so a shell handler runs before views made later and must use `cx.defer_in` to read their state for the same event.
- Source: TASK-198 fix round 1, `crates/app/src/shell.rs`.
- Use next time: defer any shell read of a child view's state that the same event updates.

### 2026-10-18 - Implement ADD TASK-220 to TASK-231

- Learned: in gpui-pre 0.3.9, `.focusable()` alone is not a Tab stop; with `track_focus`, set it on the handle (`cx.focus_handle().tab_stop(true)`). A `Visibility::Hidden` element registers no Tab stop, so hide focusable controls by opacity.
- Source: TASK-229 fix rounds 1 and 2, `packages/desktop-gpui/plugins/board/src/card.rs`.
- Use next time: when a check depends on keyboard focus, verify that Tab actually reaches the element before testing its focus style.
- Learned: a spread `BoxShadow` paints a filled quad under a transparent element, so a focus ring made from it shows as a solid box. `in_focus` means the element is inside the focused element, not that it holds focus.
- Source: TASK-229 UI checks Run C and Run D.
- Use next time: draw a focus ring as an absolute child with a border and no fill; test "holds focus" with a focus handle at render.
- Learned: adding a field to a client struct breaks struct literals in other crates' tests; plan which TASK fixes each literal, and keep `cargo build` (no tests) green in between.
- Source: TASK-220, TASK-222, TASK-231.
- Use next time: `rg "StructName \{"` across the workspace before adding a field.

### 2026-10-18 - Implement 20261018-gpui-polish-and-fixes

- Learned: the GPUI window rem size is 16 x zoom, not the 14 px body font. A gap converted with `FONT_BASE` drew 27.4 px instead of 24.
- Source: TASK-233 fix round 1, `packages/desktop-gpui/crates/timeline/src/lib.rs` (`window_rems`).
- Use next time: convert px to rems with `bachuc_theme::rem_size(cx)`; when a source read and a screenshot disagree, model every measured gap before you trust the source table.
- Learned: a daemon plugin timeline item reaches a client only when its hello lists `plugin_timeline_items`.
- Source: TASK-241, `packages/desktop-gpui/crates/client/src/connection.rs`, `packages/server/src/server/session.ts` (`supportsTimelineItem`).
- Use next time: when a daemon item is missing in GPUI, compare the hello capabilities first.
- Learned: text answers a taffy min-content probe with its unwrapped width, so a flex item that holds text needs `min_w_0` or a definite width. Measure a menu gap from the trigger's own bounds, not from its row.
- Source: TASK-232 and TASK-242 outcomes.
- Use next time: add `min_w_0` to every flex text column; read popup gaps against the button.
