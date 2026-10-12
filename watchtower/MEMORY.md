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
- Avoid restarting TASK or Q-IDs at 001 in a new plan, and avoid per-plan or per-kind ID prefixes, while one repo has one `watchtower/` folder. Reason: [ADR-0011](watchtower/decisions/ADR-0011-repo-wide-ids-over-per-plan-ids.md).

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
