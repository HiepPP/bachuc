# TASK-002 Outcome

## Outcome

Status: DONE

Changed:

- Added [packages/app/src/components/sidebar/active-host-switch.tsx](packages/app/src/components/sidebar/active-host-switch.tsx). It uses `HostPicker` with `includeAllHost`, shows the active label, and shows an amber dot for hidden hosts that need attention.
- Rendered it above `SidebarNavRows` on desktop and mobile in [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx).

Contract:

- Picking a host opens `/h/<serverId>`. The host index restores that host's remembered workspace.

Verified:

- `npm run typecheck` -> exit 0. `npm run lint` -> 0 errors.
- `rg -n "useHosts\(" packages/app/src/components/left-sidebar.tsx` -> one match in `SidebarHostPicker`.
- Verify 2026-09-30T17:40: `npm run typecheck` (all workspaces) -> exit 0; `npm run lint` on 27 changed files -> 0 errors; 8 test files -> 81 passed.
- Manual check in Paseo Dev, 2026-09-30 (cua_repl): after adding macmini over SSH, the switch shows "All hosts", "hieps-MacBook-Air-2.local", and "Hieps-Mac-mini.local". Picking a host changes the label. An amber dot shows on MacBook while a macmini agent needs attention. After restart, the label is "Active host: hieps-MacBook-Air-2.local. 1 other host needs attention".
- With one host, the switch was hidden, as designed.

Anti-goal:

- Before changes: `useHosts()` in `SidebarHostPicker`; 2026-09-30.
- Final: still there, line 387; 2026-09-30.
- Verification read: same value; checks run 2026-09-30T17:40.
- Result: PASS.
