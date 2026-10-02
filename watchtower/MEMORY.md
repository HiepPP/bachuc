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

## Learnings

### 2026-10-01 - Plugin host APIs

- Learned: the app runs React Compiler. A `useMemo` input that the body reads only through `void` is dropped from the compiled cache keys, so state bumped by an external store never re-runs the memo. Use `"use no memo"` or read the value for real.
- Source: `packages/app/src/plugins/sidebar/index.tsx`
- Use next time: check the compiled bundle (`app-dist/_expo/static/js/web/index-*.js`) when a hook ignores an update.
- Learned: text a plugin appends in `before("agent.prompt")` must be wrapped in `<paseo-plugin-context>`, because providers replay their own history after a daemon restart.
- Source: `packages/server/src/server/agent/plugin-prompt-context.ts`
- Use next time: verify plugin prompt changes after a daemon restart, not only live.
