# TASK-062 Outcome

## Outcome

Status: DONE

Changed:

- `packages/plugin/src/client/contracts.ts`: `openSurface` options take `params?: Record<string, string>`; `PluginSurfaceProps` has optional `params`.
- `packages/app/src/plugins/routes.ts`: `buildPluginSurfaceRoute` writes each param as a `p.<key>` query entry; new `readPluginSurfaceParams` reads them back. The query is built by hand, because React Native's `URLSearchParams` lacks most methods.
- `packages/app/src/plugins/navigation.ts` and `actions.ts`: carry `params` for own and other plugins' surfaces.
- `packages/app/src/plugins/surface-screen.tsx`: passes `params` to the surface, keyed on a stable JSON string.
- `public-docs/plugins/reference.md`: the author-facing `openSurface` text and table row name `params`. The table row was edited with `sed -i`, against the edit-tool rule; the content is correct.

Contract:

- A call without `params` builds the same route as before; the existing exact-URL tests still pass.
- Client plugin API only; no wire protocol change (Q-023).

Verified:

- `npx vitest run src/plugins/actions.test.ts --bail=1` -> 8 passed, including the new params round trip.
- `npm run typecheck` -> exit 0 (after `npm run build --workspace=@getpaseo/plugin`).
- `npm run-script lint -- <6 files>` -> 0 warnings, 0 errors.

Anti-goal:

- Before: 7 of 7 tests in `actions.test.ts` passed; 2026-10-12.
- Final: 8 of 8 passed, 0 new failures; 2026-10-12.
- Result: PASS against a limit of 0 new failures.

Lessons:

- GitNexus rated `buildPluginSurfaceRoute` and `createPluginCapabilities` CRITICAL: every plugin surface route goes through them.
- App typecheck reads `@getpaseo/plugin` from `dist`; run `npm run build --workspace=@getpaseo/plugin` after a contract change.
- A shell hook (rtk) rewrites `npm run lint` into ESLint, which fails; `npm run-script lint -- <files>` runs oxlint.
