# TASK-006 Plugin pages follow the active machine

Group: E (plugin sidebar and surface files)
Class: code

## Brief

Goal: Plugin sidebar items and plugin pages such as Board open on the active machine, without their own host switcher.

Change: plugin target picked by route, then remembered host -> route, then active machine, then remembered host; host switcher hidden while a machine is active.

Boundaries:

- Board reads `PluginHostProps.host.id` from the route. Do not change the plugin SDK in this plan.
- When a machine is active and the plugin is not installed on it, hide that plugin sidebar row.
- In "All machines" mode keep `PluginHostSwitcher` and the current target order.

How:

- In [packages/app/src/plugins/sidebar-items.tsx](packages/app/src/plugins/sidebar-items.tsx), add the active machine to `selectTarget`, and return `null` for the row when the active machine has no target.
- In [packages/app/src/plugins/surface-screen.tsx](packages/app/src/plugins/surface-screen.tsx), hide `PluginHostSwitcher` when a machine is active.

Files:

- [packages/app/src/plugins/sidebar-items.tsx](packages/app/src/plugins/sidebar-items.tsx) (target order and hide rule)
- [packages/app/src/plugins/surface-screen.tsx](packages/app/src/plugins/surface-screen.tsx) (hide host switcher)

Expected result:

- Machine A active: the Board row opens `/h/A/plugin/...` and Board shows A data.
- Machine A active: the Board page has no host switcher.
- "All machines" active: the switcher shows when the plugin runs on 2 or more hosts.

Anti-goal: Plugin host navigation contract; tripwire; limit [packages/app/src/plugins/host-navigation.test.ts](packages/app/src/plugins/host-navigation.test.ts) and [packages/app/src/plugins/host-navigation.test.tsx](packages/app/src/plugins/host-navigation.test.tsx) pass unchanged; read before changes and at completion.

## Verify

- `npx vitest run packages/app/src/plugins/host-navigation.test.ts packages/app/src/plugins/host-navigation.test.tsx --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/plugins/sidebar-items.tsx packages/app/src/plugins/surface-screen.tsx` -> exit 0.
- Manual check in the Paseo Dev app (human): with a machine active, Board shows that machine and no host switcher.
