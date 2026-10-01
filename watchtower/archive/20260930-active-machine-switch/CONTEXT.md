# Plan Context

## Shared Context

- The app gets its host list from `useHosts()` in [packages/app/src/runtime/host-runtime.ts](packages/app/src/runtime/host-runtime.ts) at line 2702. There are 50 non-test call sites.
- Call sites split into two sets. "Follow the active machine" sites list or aggregate hosts. "See every host" sites look up a host by `serverId`, or manage hosts in Settings and pairing.
- Workspace layout is keyed as `${serverId}:${workspaceId}` in [packages/app/src/stores/workspace-layout-store.ts](packages/app/src/stores/workspace-layout-store.ts). Tabs of a hidden machine stay on disk and come back when that machine is picked again.
- The sidebar already has a multi-host filter `hostFilters` in [packages/app/src/stores/sidebar-view-store.ts](packages/app/src/stores/sidebar-view-store.ts). It stays for the "All machines" mode.
- `HostPicker` in [packages/app/src/components/hosts/host-picker.tsx](packages/app/src/components/hosts/host-picker.tsx) already supports `includeAllHost` with the `ALL_HOSTS_OPTION_ID` option.
- The last workspace selection holds one entry only, in [packages/app/src/stores/last-workspace-selection.ts](packages/app/src/stores/last-workspace-selection.ts).
- Plugins cannot import app stores, because [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts) allowlists client modules. Plugins follow the host in the route.
- Repo rules: run `npm run typecheck` and `npm run lint` after changes. Run only the changed test files with `npx vitest run <file> --bail=1`. Never run the full suite.
- Never restart or stop the stable daemon on port 6767. The Paseo Dev app uses port 6770 and home `~/.paseo-dev`.
- Do not commit, push, or switch branches unless the user asks.

## Decisions

- Q1: Attention from a hidden machine still shows. The switch shows a badge for it. The favicon keeps counting every host.
- Q2: Keep an "All machines" option. `activeServerId: null` means all machines, and it is the default.
- Q3: Do not close tabs or panels of a hidden machine. They only leave the lists.
- Q4: When the route host differs from the active machine, switch the active machine to the route host. Do nothing when "All machines" is active.
- Active machine wins over the sidebar `hostFilters`. When a machine is active, the sidebar shows only that machine and hides the host filter section.
- Settings, pairing, add-host, diagnostics, and lookups by `serverId` keep `useHosts()`.

## Open Decisions

- None.

## References

- [packages/app/src/runtime/host-runtime.ts](packages/app/src/runtime/host-runtime.ts)
- [packages/app/src/stores/sidebar-view-store.ts](packages/app/src/stores/sidebar-view-store.ts)
- [packages/app/src/stores/navigation-active-workspace-store/index.ts](packages/app/src/stores/navigation-active-workspace-store/index.ts)
- [packages/app/src/components/hosts/host-picker.tsx](packages/app/src/components/hosts/host-picker.tsx)
- [docs/development.md](docs/development.md)
- `npm run build:desktop`
