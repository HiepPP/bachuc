# Watchtower Memory

## Purpose

Read this file before creating or implementing Watchtower tasks in this repo.

This file holds long-term intent. It keeps task planning aligned across sessions.

## Core Intent

- This checkout is the user's own Paseo fork. The user's plugins live in `hiep-plugins/plugins/`.
- Two instances exist: release, this repo built and installed as the daily app (`PASEO_HOME=~/.paseo`, port 6767), and live, run from source (port 6768). Stock Paseo from the upstream GitHub is no longer used. Plugins install into release and live.

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

- Avoid running sidebar host filters together with an active host while one host is active. Reason: [ADR-0001](watchtower/decisions/ADR-0001-active-host-over-host-filters.md).
- Avoid an offline-only CSP for HTML reply frames while the owner wants agent pages that load CDN libraries and call APIs. Reason: [ADR-0002](watchtower/decisions/ADR-0002-networked-html-replies-over-offline-only.md).

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
- Source: `packages/server/src/server/session.test.ts` ("cancel_agent_request reports refusal only through its response"); TASK-003 outcome.
- Use next time: run `session.test.ts` after any change to `packages/server/src/server/agent/lifecycle-command.ts`.

### 2026-10-07 - Autorun 20261006-t3code-orchestration-port

- Learned: after a `packages/plugin` type change, the app typecheck reads stale declarations until the SDK is rebuilt.
- Source: TASK-006 and TASK-010 outcomes.
- Use next time: run `npm run build:plugin` before `npm run typecheck`.
- Learned: app tests that load Expo fail from the repo root with `__DEV__ is not defined`.
- Source: TASK-006 and TASK-010 runs.
- Use next time: run app vitest files from `packages/app`, with `--project browser` for `*.browser.test.ts`.
- Learned: the agent stream view mounts before its authoritative history arrives, and the vitest Lucide stub lacks many icons, such as `Quote`.
- Source: `packages/app/src/panels/agent-panel.tsx`; `packages/app/test-stubs/lucide-react-native.ts`; TASK-010 outcome.
- Use next time: wait for `isAuthoritativeHistoryReady` before checking loaded items, and use `Blocks` as the icon in plugin tests.
