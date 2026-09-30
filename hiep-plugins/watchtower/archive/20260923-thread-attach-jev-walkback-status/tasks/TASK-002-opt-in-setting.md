# TASK-002 Opt-in setting and settings screen

Group: A (settings files and the client entry)
Class: code

## Brief

Goal: Add a host setting that turns Jev support on. It is off by default. Users can switch it in Settings -> Plugins.

Change: no setting -> `thread-context` settings with `jevEnabled: false` and a switch screen.

How:

- Add `threadContextSettings` with `defineSettings`. Use id `thread-context`, scope `host`, version 1, and schema `{ jevEnabled: boolean, default false }`. Follow [plugins/thread-janitor/shared/settings.ts](plugins/thread-janitor/shared/settings.ts).
- Do not edit `index.server.ts`. TASK-003 registers the settings on the server.
- Add a settings screen with one `SettingsSwitch`. Its hint says that reply text goes to Vercel AI Gateway when the switch is on. Follow [plugins/board/client/orb-settings.tsx](plugins/board/client/orb-settings.tsx) for loading, error, and invalid states.
- Register the screen in the client entry with `client.addSettingsScreen`, like [plugins/board/index.client.tsx](plugins/board/index.client.tsx).

Files:

- [plugins/thread-context-attach/shared/settings.ts](plugins/thread-context-attach/shared/settings.ts) (new settings definition)
- [plugins/thread-context-attach/client/settings.tsx](plugins/thread-context-attach/client/settings.tsx) (new settings screen)
- [plugins/thread-context-attach/index.client.tsx](plugins/thread-context-attach/index.client.tsx) (register the screen)
- [plugins/thread-context-attach/package.json](plugins/thread-context-attach/package.json) (add `client` to the format and lint paths)

Expected result:

- A fresh install reads `jevEnabled: false`.
- The switch saves the value. TASK-003 reads it on the server.

## Verify

- `cd plugins/thread-context-attach && npm run typecheck && npm test` -> exit 0.
- `cd plugins/thread-context-attach && S=lint; npm run $S` -> exit 0.
- Manual, needs the Paseo app, after TASK-003 registers the settings: Settings -> Plugins shows the switch as off. Turning it on persists after `paseo plugin reload thread-context-attach`.
