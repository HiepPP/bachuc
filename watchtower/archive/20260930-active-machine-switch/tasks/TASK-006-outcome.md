# TASK-006 Outcome

## Outcome

Status: DONE

Changed:

- `selectTarget` in [packages/app/src/plugins/sidebar-items.tsx](packages/app/src/plugins/sidebar-items.tsx) pins to the active host. The row is hidden when the active host lacks the plugin, and it was split into `PluginSidebarTargetRow` to keep hook order.
- `PluginHostSwitcher` is hidden while a host is active.

Contract:

- The plugin SDK did not change. Board follows the route host.

Verified:

- `npx vitest run src/plugins/host-navigation.test.ts src/plugins/host-navigation.test.tsx --bail=1` -> 10 passed.
- `npm run typecheck` -> exit 0. `npm run lint` -> 0 errors.
- Verify 2026-09-30T17:40: `npm run typecheck` (all workspaces) -> exit 0; `npm run lint` on 27 changed files -> 0 errors; 8 test files -> 81 passed.
- Manual check in Paseo Dev, 2026-09-30 (cua_repl): with MacBook active, Board opens `h/srv_A1fyrcLO0oqM/plugin/board/...` and shows only MacBook projects. With macmini active, it opens `h/srv_qcIOi81KL8oC/plugin/board/...` and shows only `baconsua`. No plugin host switcher appears.

Anti-goal:

- Before changes: host-navigation tests pass; 2026-09-30.
- Final: 10 passed; 2026-09-30.
- Verification read: same value; checks run 2026-09-30T17:40.
- Result: PASS.
