# Agent instructions

## Scope

This repository contains independent Paseo plugins, not the Paseo application.
Keep changes small, preserve unrelated work, and do not commit or push unless requested.
Never store API keys, local config files, credentials, or real user prompts in Git.

Paseo supervises coding agents on a host daemon. Its installed app can load local plugins
without rebuilding Paseo or publishing this repository. Git push and plugin reload are
separate operations: pushing does not update an installed directory plugin.

## Repository layout

Each plugin lives in `plugins/<plugin-id>/` and owns its manifest, dependencies, scripts,
and README. Do not add root workspaces or shared packages without a concrete need.

```text
plugins/<plugin-id>/
  paseo-plugin.json
  package.json
  package-lock.json
  tsconfig.json
  index.server.ts       # Optional daemon entry
  index.client.tsx      # Optional app entry
  server/               # Server runtime modules
  client/               # Client runtime modules
  shared/               # Cross-runtime contracts and plain values
```

At least one runtime entry is required. Keep imported runtime modules in `server/`,
`client/`, or `shared/`, not alongside the entries. Standalone test/benchmark scripts
may live separately when they are not imported by plugin entries.

## Documentation and compatibility

Before adopting an API, check the target daemon/app version and the matching documentation:

- Documentation index: https://paseo.sh/llms.txt
- Quickstart: https://paseo.sh/docs/plugins.md
- Plugin reference: https://paseo.sh/docs/plugins/reference.md
- Provider plugins: https://paseo.sh/docs/plugins/providers.md

A checkout can contain unreleased APIs. Do not assume every documented capability exists
in the installed host. Declare the supported range in `paseo-plugin.json` under
`requirements.paseo`; check client compatibility when adding UI.

## Explore Paseo source

Before designing a non-trivial plugin, read how the host implements the API you will use.
The local checkout is `~/Projects/paseo`. Treat it as read-only: never edit, commit, or
switch branches there without explicit permission.

1. Compare versions: `git -C ~/Projects/paseo describe --tags` against `paseo --version`.
   If they differ, say so and prefer behavior verified against the installed host.
2. Read docs: `public-docs/plugins/reference.md` (plus `index.md`, `providers.md`,
   `migration.md` there) and `docs/plugins.md`. Paseo's own `CLAUDE.md` and
   `docs/architecture.md` explain host structure.
3. Read the closest `plugin-examples/<name>/` and copy its patterns.
4. Read host implementation in `packages/`: `plugin/` (SDK contracts and types),
   `server/src/server/plugins/` (daemon plugin host), `app/src/plugins/` (client
   contributions and UI), `protocol/` (message schemas).
5. Use GitNexus repo `paseo` for flows and callers, and `rg` for text. Check index freshness
   first; verify graph results against source.

Record host facts a design depends on with the Paseo version or commit SHA, for example in the
plugin's spec or README.

## Plugin capabilities

These are contributions that can be combined, not mutually exclusive plugin types.
Verify exact signatures and version availability before implementation.

| Capability                   | Main API or mechanism                            | Example                                                 |
| ---------------------------- | ------------------------------------------------ | ------------------------------------------------------- |
| Agent lifecycle automation   | `server.on`, `server.before`                     | Inject MCP or environment, react to turn completion     |
| Backend operations           | `defineRpc`, `server.handle`, client `useRpc`    | Call an external API without exposing credentials to UI |
| App pages and navigation     | `addSurface`, `addSidebarItem`                   | Service dashboard                                       |
| Workspace panels             | `addWorkspacePanel`                              | Workspace-specific inspector                            |
| Quick actions                | `addCommandCenterItem`, `addSlashCommand`        | User-triggered workflow                                 |
| Header and composer controls | Header buttons, `addComposerPill`                | Contextual action beside the prompt                     |
| Timeline presentation        | `addTimelineTransformer`, `addTimelineRenderer`  | Render structured tool output                           |
| Timeline additions           | `paseo.agents.ref(id).timeline.append`           | Plugin-owned status or result row                       |
| Attachments                  | `addAttachmentSource` with backend RPC           | Attach an issue or document to a prompt                 |
| Themes                       | `addTheme`                                       | App color palette                                       |
| Settings                     | Settings screens and persisted values            | Plugin preferences                                      |
| Agent providers              | `server.registerProvider`, direct or ACP adapter | Integrate another agent runtime                         |

Use the Paseo SDK for supported workspace and agent operations. A plugin is not a
supported way to replace arbitrary application internals.

## Runtime boundaries

- `index.server.ts` runs in a daemon subprocess; `index.client.tsx` runs in connected apps.
- Keep Node APIs, filesystem access, credentials, and vendor API calls on the server.
- Client code cannot import `server/` or Node modules. Server code cannot import `client/`.
- Use shared Zod contracts for RPC. Keep `shared/` free of runtime-specific imports.
- Use host-supported modules and UI primitives; do not assume arbitrary client npm packages work.
- Follow the reference cross-platform rules; do not assume every client is a desktop browser.
- Return cleanup from contribution functions for subscriptions, timers, sockets, and other resources.
- Plugins are trusted, unsandboxed code. Install only authorized, inspected code.

## Create and install

Inspect the target directory first; reuse existing work and never overwrite a plugin blindly.
From the repository root, scaffold a new plugin with a unique lowercase hyphenated ID:

```sh
paseo plugin init "$PWD/plugins/<plugin-id>"
cd plugins/<plugin-id>
npm install
npm run typecheck
# Run the plugin's declared lint and focused test scripts, if present.
PASEO_HOME=~/.paseo-dev paseo plugin install "$PWD" --id <plugin-id>
PASEO_HOME=~/.paseo-dev paseo plugin ls <plugin-id> --json
```

`init` scaffolds files but does not install dependencies. Keep lockfiles committed.
Document required configuration, commands, permissions, and provider limitations in the plugin README.
Add the plugin to the root README catalog.

Install, reload, and remove these plugins only in Paseo Dev (`PASEO_HOME=~/.paseo-dev`,
daemon `127.0.0.1:6770`). Never install them into the stable Paseo (`~/.paseo`, port `6767`).
Prefix every `paseo plugin ...` and `paseo daemon ...` command with `PASEO_HOME=~/.paseo-dev`,
and confirm with `PASEO_HOME=~/.paseo-dev paseo daemon status --json` that `home` is
`~/.paseo-dev` before installing. If plugins are disabled, explain the unsandboxed access and obtain permission
before enabling them. Never print the daemon config wholesale because it can contain secrets.

## Update and verify

After edits, run the plugin's formatter, typecheck, lint, and affected tests. Then:

```sh
PASEO_HOME=~/.paseo-dev paseo plugin reload <plugin-id>
PASEO_HOME=~/.paseo-dev paseo plugin ls <plugin-id> --json
PASEO_HOME=~/.paseo-dev paseo plugin logs <plugin-id>
```

Require `running` with no load error and exercise the changed contribution. Reload recompiles
local source; it does not require a Paseo rebuild. A failed reload stays failed: inspect the
error and fix it. Never restart the main daemon to load plugin changes.

Verify UI changes in the light theme only. The user works in light theme, so do not
switch the app to dark theme or test dark theme unless explicitly asked.

For `Transport not connected (status: disconnected)` during plugin RPC, manually reload
the affected plugin. Local `~/.zshrc` defines `prefresh` as `paseo plugin reload`:

```sh
prefresh <plugin-id>
# Agents must load interactive zsh aliases; example:
zsh -ic 'prefresh watchtower-board'
```

If the alias is unavailable, use `paseo plugin reload <plugin-id>` directly. Follow the
typecheck gate above before reloading. Verify status, logs, and the failed action afterward;
`running` alone does not prove RPC recovery. This is manual recovery, not a permanent
transport fix. Do not restart the daemon or change Paseo source for this workaround.

Directory installs depend on the source path remaining available. When moving a plugin,
update its registration; installing under an already configured ID fails. The CLI's
`plugin remove` removes registration, not source, so remove/reinstall can rebind the ID.
Preserve old MCP executable paths used by existing agents, for example with local ignored
symlinks. Verify both the new registration and old entry paths after migration.

## Jev-specific knowledge

`plugins/jev-evaluator/` exposes `jev_evaluate` through a stdio MCP server. Jev is an
evaluation model, not a coding/chat agent; do not configure it as a Responses chat model.
The main Codex/Claude agent calls the tool with state and typed boolean/choice/score questions.

The plugin hooks `agent.create` for built-in Codex and Claude providers. New agents created
through Paseo receive the MCP entry; existing agents do not gain it retroactively.
Provider-internal subagents are not covered by this hook. Existing `mcpServers.jev` is preserved.
Install this plugin under its required ID `jev-evaluator`.

The MCP reads the Typesafe key from `$PASEO_HOME/typesafe-ai.json` (`apiKey`, mode `0600`) at invocation time. Keep the key
out of prompts, Git, tool output, and agent MCP environment entries. See the plugin README for
the exact config field and evaluation contract.

Run from `plugins/jev-evaluator/`:

```sh
npm run format
npm run typecheck
npm run lint
npm test
npm run test:benchmark
```

`npm run smoke` and `npm run benchmark` call the real API and consume quota; they are not
routine offline checks. Benchmark labels must remain separate from model inputs. Preserve
wrong answers and API failures; do not rerun answers to improve reported scores. Model
confidence is not proof of correctness. Local artifacts and dependencies stay ignored.
