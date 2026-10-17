# TASK-192 Docs for GPUI plugin clients

Group: T (writes `CLAUDE.md`, `docs/plugins.md`, and `packages/desktop-gpui/docs/parity-audit.md`)
Class: docs

## Brief

Goal: Make the docs say where a plugin's GPUI client lives and how it loads, and bring the parity audit up to date with the plan's results.

Change: the docs say a plugin is `hiep-plugins/plugins/<id>` with `index.client.tsx` running in every app, and the audit lists plugin surfaces and the New workspace screen as out of scope -> the docs name the native Rust client crate, its rules, and its rebuild need, and the audit shows the new state.

Design: [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md); the "Writing docs" and "Doc voice" rules in [CLAUDE.md](CLAUDE.md); the plugin host contract in [watchtower/tasks/TASK-170-plugin-host-crate-and-registry.md](watchtower/tasks/TASK-170-plugin-host-crate-and-registry.md) Boundaries.

Boundaries:

- Integrate, do not append. Rewrite the sentences that are now wrong, where they stand. One fact lives in one doc; every other mention is a link.
- [CLAUDE.md](CLAUDE.md): rewrite the "Code a plugin" bullet (line 24). A plugin's server part stays in `hiep-plugins/plugins/<plugin-id>`. Its GPUI client is the crate `packages/desktop-gpui/plugins/<plugin-id>`, named `bachuc-plugin-<plugin-id>`. Its React client is frozen. Add that `plugin reload` reloads only the server part; a GPUI client change needs a GPUI build. Update the `docs/plugins.md` row of the docs table to mention GPUI clients. Link to the new `docs/plugins.md` section; do not repeat its rules.
- [docs/plugins.md](docs/plugins.md): rewrite the intro sentence "Paseo executes `index.server.ts` in a subprocess and `index.client.tsx` in every connected app" (line 5) so it covers both clients. Add one section, "GPUI native clients", next to "Contribute behavior and UI" (line 172). It holds only what the code cannot tell a reader:
  - The crate depends on `bachuc-plugin-host` and never on a view crate, so no dependency cycle forms.
  - It declares `gpui-kit` with no `features` key, because another feature set makes a second GPUI build.
  - It reaches its server part only through plugin RPCs. Settings use the `settings.<id>.read`, `.write`, and `.reset` RPCs.
  - It shows only when the active host's `plugin.list` reports it enabled and running.
  - The app registers crates in `crates/app/src/plugins.rs` in plugin id order, the daemon catalog order; that order decides each "first wins" rule.
  - A plugin with no client part needs no crate.
- [packages/desktop-gpui/docs/parity-audit.md](packages/desktop-gpui/docs/parity-audit.md): update the date line. Remove "Plugin surfaces" and "New workspace screen" from "Out of scope surfaces". Rewrite the Composer rows and the "Ranked mismatches" items that TASK-168 to TASK-191 changed, using each outcome's UI check result. A row with no new check keeps its old value, or becomes UNVERIFIED when the old value no longer holds. Add rows for the track strip, the New workspace screen, and the plugin pills.
- Write a fact only when an outcome file or the code states it. Read [watchtower/tasks](watchtower/tasks) `TASK-168-outcome.md` to `TASK-191-outcome.md` first.
- Doc voice: plain and short, second person, no setup-and-punchline sentences, and none of the banned words of CLAUDE.md "Doc voice".

Files:

- [CLAUDE.md](CLAUDE.md) (the "Code a plugin" bullet, the plugin reload note, and the `docs/plugins.md` table row)
- [docs/plugins.md](docs/plugins.md) (the intro sentence and the new "GPUI native clients" section)
- [packages/desktop-gpui/docs/parity-audit.md](packages/desktop-gpui/docs/parity-audit.md) (date, Composer rows, ranked mismatches, out-of-scope table)

Interfaces:

- Consumes: the app build that holds every plugin client from TASK-191

Expected result:

- A reader of CLAUDE.md learns where a plugin's GPUI client lives and that it needs a GPUI build, in the bullet that already defines "Code a plugin".
- `docs/plugins.md` has one "GPUI native clients" section, and its intro no longer says the React client runs in every app.
- The audit no longer lists plugin surfaces or the New workspace screen as out of scope.

Anti-goal: CLAUDE.md, which every session loads, grows by at most 4 lines; tripwire: `git diff --numstat -- CLAUDE.md` shows added minus removed lines of 4 or fewer; read at completion.

## Verify

- `npm run format:check:files -- CLAUDE.md docs/plugins.md packages/desktop-gpui/docs/parity-audit.md` -> exit 0 (three separate arguments; "Expected at least one target file" is a failure, `MEMORY.md` 2026-10-16)
- `rg -n "packages/desktop-gpui/plugins" CLAUDE.md docs/plugins.md` -> at least one match in each file
- `rg -c "^## GPUI native clients" docs/plugins.md` -> 1
- `rg -n "in every connected app" docs/plugins.md` -> no match
- `rg -n "^\| (Plugin surfaces|New workspace screen) +\| out of scope" packages/desktop-gpui/docs/parity-audit.md` -> no match
- `git diff --numstat -- CLAUDE.md` -> added minus removed is 4 or fewer
- Manual read: each new markdown link in the three files opens an existing file
