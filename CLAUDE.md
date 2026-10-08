# CLAUDE.md

Paseo is a mobile app for monitoring and controlling your local AI coding agents from anywhere. Your dev environment, in your pocket. Connects directly to your actual development environment — your code stays on your machine.

**Supported agents:** Claude Code, Codex, GitHub Copilot, OpenCode, and Pi.

## Repository map

This is an npm workspace monorepo:

- `packages/server` — Daemon: agent lifecycle, WebSocket API, MCP server
- `packages/app` — Mobile + web client (Expo)
- `packages/cli` — Docker-style CLI (`paseo run/ls/logs/wait`)
- `packages/relay` — E2E encrypted relay for remote access
- `packages/desktop` — Electron desktop wrapper
- `packages/website` — Marketing site (paseo.sh)
- `hiep-plugins` — The user's own Paseo plugins (`hiep-plugins/plugins/*`), with their own `AGENTS.md`

The fork's product brand is Bachuc, and its release home is `~/.bachuc`. Other internal names, such as the `paseo` CLI and `PASEO_*` env vars, stay `paseo` per [ADR-0009](watchtower/decisions/ADR-0009-bachuc-home-over-paseo-home.md).

This fork is dedicated to one user; it is not meant for other Paseo users. Extend it through plugins:

- "Code a plugin" means editing `hiep-plugins/plugins/<plugin-id>`, not core plugin code in `packages/*` or `plugin-examples/`. Read `hiep-plugins/AGENTS.md` first. `hiep-plugins` is the only plugin source; the old repo `~/Projects/hiep-paseo-plugin` is frozen, so never edit it or install from it.
- When a plugin cannot reach deep enough, add the smallest generic host API in `packages/*` that unblocks it, then build the feature in the plugin. Keep host changes thin (a hook, a contribution point, a context field), never the feature itself, so upstream merges stay cheap.
- Plugins may require the fork version; supporting stock Paseo is not a goal.
- Install and reload these plugins in **release** (`PASEO_HOME=~/.bachuc paseo plugin ...`) and **live** (`npm run cli -- plugin ...`).

## Docs

`docs/` is the source of truth for system-level and process-level knowledge. **"The docs", "check the docs", or "check the X docs" always mean this directory — not the web.** Look here before fetching anything online; the docs capture gotchas and conventions you cannot derive from the code or external sources.

At the start of non-trivial work, list `docs/` and skim anything relevant to the task.

| Doc                                                                  | What's in it                                                                                                                   |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| [docs/product.md](docs/product.md)                                   | What Paseo is, who it's for, where it's going                                                                                  |
| [docs/architecture.md](docs/architecture.md)                         | System design, package layering, WebSocket protocol, agent lifecycle, data flow                                                |
| [docs/agent-lifecycle.md](docs/agent-lifecycle.md)                   | Agent states, parent/child relationships, archive semantics, tabs vs archive, subagents track                                  |
| [docs/data-model.md](docs/data-model.md)                             | File-based JSON persistence, Zod schemas, atomic writes, no migrations                                                         |
| [docs/glossary.md](docs/glossary.md)                                 | Authoritative terminology — UI label wins, no synonyms                                                                         |
| [docs/coding-standards.md](docs/coding-standards.md)                 | Type hygiene, error handling, state design, React patterns, file organization                                                  |
| [docs/design.md](docs/design.md)                                     | Design system — tokens, buttons, hierarchy, density, alignment rails, states, what's forbidden                                 |
| [docs/forms.md](docs/forms.md)                                       | Form architecture — non-React form model, form kit, load-state gating; the schedule form is the golden example                 |
| [docs/hover.md](docs/hover.md)                                       | Hover — the canonical pattern (plain View + onPointerEnter/Leave, separate inner Pressable) and the three ways agents break it |
| [docs/unistyles.md](docs/unistyles.md)                               | Unistyles gotchas — `useUnistyles()` is forbidden, alternatives in order                                                       |
| [docs/floating-panels.md](docs/floating-panels.md)                   | Anchored popovers — Portal/Modal escape for Android, lifecycle gates, keyboard-shared-value, status-bar offset, the flash      |
| [docs/menus.md](docs/menus.md)                                       | The menu engine — popover vs sheet, submenu pages, hover intent, when a decision earns a submenu                               |
| [docs/expo-router.md](docs/expo-router.md)                           | Expo Router route ownership, startup restore, and native blank-screen gotchas                                                  |
| [docs/file-icons.md](docs/file-icons.md)                             | Material icon theme integration for the file explorer                                                                          |
| [docs/providers.md](docs/providers.md)                               | Adding a new agent provider end-to-end                                                                                         |
| [docs/forge-providers.md](docs/forge-providers.md)                   | Adding a git forge: registry/manifest, drop-in checklist, self-host/GHES, the two facts tiers                                  |
| [docs/custom-providers.md](docs/custom-providers.md)                 | Custom provider config: Z.AI, Alibaba/Qwen, ACP agents, profiles, custom binaries                                              |
| [docs/plugins.md](docs/plugins.md)                                   | Local plugin manifest, directory source config, RPCs, native surfaces, and attachment sources                                  |
| [docs/service-proxy.md](docs/service-proxy.md)                       | Service proxy: exposing workspace scripts at public URLs, DNS setup, reverse proxy config                                      |
| [docs/development.md](docs/development.md)                           | Dev server, build sync gotchas, CLI reference, agent state, Playwright MCP                                                     |
| [docs/rpc-namespacing.md](docs/rpc-namespacing.md)                   | WebSocket RPC naming convention — dotted namespaces and `.request`/`.response` pairs                                           |
| [docs/protocol-compatibility.md](docs/protocol-compatibility.md)     | Why app/daemon versions drift, protocol vs feature contract, capability gating, COMPAT tagging                                 |
| [docs/protocol-validation.md](docs/protocol-validation.md)           | zod-aot generated inbound WebSocket validation, patched compiler regressions, schema-purity rules                              |
| [docs/permissions.md](docs/permissions.md)                           | Semantic daemon permissions, principals, credentials, pairing invitations, and Hub authority                                   |
| [docs/terminal-performance.md](docs/terminal-performance.md)         | Terminal latency pipeline, coalescing/backpressure invariants, benchmark + perf spec usage                                     |
| [docs/agent-stream-performance.md](docs/agent-stream-performance.md) | Assistant text pipeline — coalescing window, paced reveal, why arrival lumps are smoothed at render                            |
| [docs/file-observation.md](docs/file-observation.md)                 | Recursive watcher ownership, Linux constraints, teardown invariants, and Parcel comparison                                     |
| [docs/testing.md](docs/testing.md)                                   | TDD workflow, determinism, real dependencies over mocks, test organization                                                     |
| [docs/qa.md](docs/qa.md)                                             | QA evidence bar for pull requests — platform matrix, version drift, performance, UI proof                                      |
| [docs/mobile-testing.md](docs/mobile-testing.md)                     | Maestro and mobile test workflows                                                                                              |
| [docs/mobile-panels.md](docs/mobile-panels.md)                       | Compact left/center/right panel ownership, worklet motion, gesture revisions, and Fabric constraints                           |
| [docs/explorer-sidebar.md](docs/explorer-sidebar.md)                 | Explorer sidebar and ordinary side-pane host contracts, lifecycle, placement, and routing preferences                          |
| [docs/ad-hoc-daemon-testing.md](docs/ad-hoc-daemon-testing.md)       | Isolated in-process daemon test harness                                                                                        |
| [docs/browser-capture-harness.md](docs/browser-capture-harness.md)   | Real-Electron browser screenshot harness and compositor-surface gotcha                                                         |
| [docs/android.md](docs/android.md)                                   | App variants, local/cloud builds, EAS workflows, version codes, F-Droid source builds and store metadata                       |
| [docs/docker.md](docs/docker.md)                                     | Running the daemon and bundled web UI in Docker, volumes, agent images, security                                               |
| [docs/release.md](docs/release.md)                                   | Release playbook, draft releases, completion checklist                                                                         |
| [docs/terminal-activity.md](docs/terminal-activity.md)               | Terminal activity indicators — source-agnostic tracker, agent hook reporting, adding a new hook provider                       |
| [SECURITY.md](SECURITY.md)                                           | Relay threat model, E2E encryption, DNS rebinding, agent auth                                                                  |
| [public-docs/hub/security.md](public-docs/hub/security.md)           | Public Hub guide — trust boundaries, untrusted triggers, provider controls, and output authority                               |

### Writing docs

- **Integrate, don't append.** Find the doc that owns the subject and rewrite the part that is now wrong. The standard failure is finishing a task and adding a paragraph to the bottom of the closest-looking doc; ten tasks later the doc is a pile of paragraphs in discovery order. `docs/custom-providers.md` is what that looks like.
- **Don't document logic.** Prose that restates code drifts from the code and loses. Write down what the code can't tell you: why something is shaped the way it is, the gotcha that cost an afternoon, conventions nothing enforces, constraints that span packages or versions. If a reader could get it in two minutes by opening the file, cut it.
- **One fact, one doc.** Every other mention is a link. If you are about to write the same paragraph in two docs, one of them is a link.
- **Respect the layers.** `CONTRIBUTING.md` and this file name things and link out. Activity docs like `docs/qa.md` and `docs/testing.md` set the bar for a kind of work. Subject docs like `docs/unistyles.md` own one thing completely. A layer never re-explains the one below it.
- **One subject per doc.** If the subject doesn't fit in a sentence, split the doc. A section per provider, vendor, or platform is a table plus one worked example.
- **Delete.** Obsolete sections go. Prefer a `packages/app/src/thing.ts:120` reference over a pasted block.
- **New doc?** Add a row to the table above and link it from the docs that should send readers there.
- Code-level facts belong in comments next to the code, not here.

### Doc voice

Plain and short. Second person. State the rule, then the reason when the reason isn't obvious. Match the doc you're editing.

Do not:

- Write a sentence to land a point. "It's not X, it's Y", "That's not a Z, that's a W", and every other setup-and-punchline shape.
- Add a clause that only asserts importance: "and that matters", "which is what keeps it working", "this is critical".
- Use "honest", "robust", "seamless", "powerful", "simply", "just", "delightful".
- Restate something you already said, in different words, for emphasis.
- Hedge with "generally", "typically", or "you may want to" when the answer is "do this".
- Clear your throat: "It's worth noting that", "In order to", "This section covers".

## Quick start

```bash
npm run dev                          # Start the dev daemon
npm run dev:app                      # Start Expo against the dev daemon
npm run dev:desktop                  # Start Electron desktop dev
npm run cli -- ls -a -g              # List all agents
npm run cli -- daemon status         # Check daemon status
npm run typecheck                    # Always run after changes
npm run lint                         # Always run after changes
npm run format                       # Auto-format with Biome
npm run format:check                 # Check formatting without writing
```

Two Paseo instances run side by side on this machine. Prompts name them **release** and **live**; each name means exactly one daemon, home, and app. "Release", "the release version", and "build" in a prompt all mean **release**: this repo built from source and installed as the daily app, at a fork version such as `hiep-1.0.0`. The upstream app from the Paseo GitHub is **stock**; it is no longer used. Cutting a version is a separate subject, in [docs/release.md](docs/release.md).

| Name        | What it is                                                  | Daemon listen    | `PASEO_HOME`      | Electron userData                      | Logs                                             |
| ----------- | ----------------------------------------------------------- | ---------------- | ----------------- | -------------------------------------- | ------------------------------------------------ |
| **release** | This repo built and installed as `/Applications/Bachuc.app` | `127.0.0.1:6767` | `~/.bachuc`       | `~/Library/Application Support/Bachuc` | `~/.bachuc/daemon.log`, `~/Library/Logs/Bachuc/` |
| **live**    | This repo run from source, no build                         | `127.0.0.1:6768` | `.dev/paseo-home` | `.dev/user-data`                       | `.dev/paseo-home/daemon.log`                     |

Commands per name:

```bash
# release — the daily app. Its daemon owns every running agent: never restart or stop it.
# Only the user installs a new build, with scripts/paseo-release.sh from Terminal.app.
PASEO_HOME=~/.bachuc paseo daemon status
PASEO_HOME=~/.bachuc paseo plugin reload <plugin-id>
# Build only when the prompt asks for a release build. This does not install it.
npm run build:desktop -- --dir -c.mac.hardenedRuntime=false -c.mac.notarize=false

# live — the default for testing a source change; also has hiep-plugins.
npm run dev            # daemon, terminal 1
npm run dev:desktop    # Metro + Electron, terminal 2
npm run cli -- daemon status
npm run cli -- plugin install "$PWD/hiep-plugins/plugins/<plugin-id>" --id <plugin-id>
npm run cli -- plugin reload <plugin-id>
```

- **Test source changes on live; do not build a release unless the prompt asks for one.** App changes reach the live Electron window within seconds, and `protocol`/`client` rebuild through `tsc --watch`. The live daemon does not restart on a server change: restart `npm run dev` yourself, and restart `npm run dev:desktop` after a `packages/desktop/src` change. `npm run build:desktop` costs over a minute per run, and its clean steps delete the `dist` folders a running live instance uses.
- **A source change reaches release only through a new build.** `scripts/paseo-release.sh` builds, installs the build as `/Applications/Bachuc.app`, and restarts the daemon on `6767`. That stops every running agent, so the script refuses to run inside Paseo; the user runs it from Terminal.app. See [Installing a release](docs/development.md#installing-a-release).
- An agent shell inside Bachuc inherits `PASEO_HOME=~/.bachuc`, and the `paseo` on `PATH` is release's CLI, so a bare `paseo ...` targets release. `scripts/dev-home.sh` ignores that value and the old `~/.paseo`, so `npm run dev`, `npm run dev:desktop`, and `npm run cli` use `.dev/paseo-home`; check that the startup banner shows `Home: …/.dev/paseo-home`.
- The app loads only the active host's plugins, so a plugin change needs `plugin reload` on the host you are looking at; reload release and live to keep them in step.
- The build output, `packages/desktop/release/mac-arm64/Bachuc.app`, is not release until it is installed. Opened in place it runs apart from release, on `~/.paseo-dev` and `127.0.0.1:6770` with userData `~/Library/Application Support/Bachuc Dev`: `packages/desktop/src/main.ts` gives `~/.bachuc` and `6767` only to the install path `/Applications/Bachuc.app`.
- The first Bachuc install cloned `~/.paseo` to `~/.bachuc` and `~/Library/Application Support/Paseo` to `Bachuc`. The old folders stay as a backup; nothing reads them, and only the user deletes them.
- Do not assume `/Applications/Paseo.app` exists. The old stock app and its data sit in `~/.paseo-switch`; nothing reads that folder, and only the user deletes it.
- Daemon state inside a home: `config.json` (settings, profiles, plugin sources), `agents/`, `projects/`, `schedules/`, `plugin-data/`, `plugin-settings/`, `desktop-attachments/`, `models/`, plus identity files `server-id` and `daemon-keypair.json`.
- `~/.paseo-dev` was cloned from `~/.paseo` on 2026-09-28, keeping its own `server-id` and keypair. Server IDs in `projects/`, `plugin-settings/`, and `plugin-data/` were rewritten to its own ID. The previous copy is at `~/.paseo-dev.bak-20260928-145243`. Electron Local Storage was not cloned, because its host registry points at `6767`.

See [docs/development.md](docs/development.md) for full setup, build sync requirements, and debugging.

## Release branches

When the user says "this goes to next", create or
retarget the PR to `next` and preserve that destination through delivery. Follow
[release branch discipline](docs/release.md#release-branch-discipline) for creating
and updating `next`, integrating it after a release, and releasing a hotfix from a tag.

## Critical rules

- **NEVER restart the main Paseo daemon on port 6767 without permission** — it manages all running agents. If you're an agent, restarting it kills your own process.
- **NEVER assume a timeout means the service needs restarting** — timeouts can be transient.
- **NEVER add auth checks to tests** — agent providers handle their own auth.
- **Before changing app routes, startup routing, remembered workspace restore, or active workspace selection, read [docs/expo-router.md](docs/expo-router.md).**
- **NEVER run the full test suite locally.** The test suites are heavy and will freeze the machine, especially if multiple agents run them in parallel. Rules:
  - Run only the specific test file you changed: `npx vitest run <file> --bail=1`
  - Never run `npm run test` for an entire workspace unless explicitly asked.
  - If you must run a broad suite, pipe output to a file and read it afterward: `npx vitest run <file> --bail=1 > /tmp/test-output.txt 2>&1` then read the file.
  - Never re-run a test suite that another agent already ran and reported green — trust the result.
  - GitHub Actions is off for this fork since 2026-10-08, so no CI run checks a change. Run each test file your change affects locally, one file at a time.
- Add tests to existing suites and reuse their npm scripts and CI jobs instead of creating feature-specific ones.
- **Always run typecheck and lint after every change.**
- **Build workspace packages before diagnosing cross-package type errors.** This repo consumes generated declarations across workspaces. If typecheck fails in a package that depends on another workspace, rebuild the owning stack first so `dist` declarations are current:
  - `npm run build:client` — rebuild protocol and client declarations.
  - `npm run build:server` — rebuild highlight, relay, protocol, client, server, and CLI when server/CLI types may be stale.
  - Do not patch inferred callback parameters or add local duplicate types just to silence stale declaration errors.
- **Run `npm run format` before committing.** This repo uses Biome for formatting. Do not manually fix formatting — let the formatter handle it.
- **Always use npm scripts for linting and formatting.** Do not run tools directly with `npx eslint`, `npx oxfmt`, `npx oxlint`, or package-local binaries. For targeted checks, pass file paths through the npm script:
  - `npm run lint -- packages/app/src/components/message.tsx`
  - `npm run format:files -- CLAUDE.md packages/app/src/components/message.tsx`
- **The protocol stays backward-compatible. Features don't have to.** Read [docs/protocol-compatibility.md](docs/protocol-compatibility.md) before touching `packages/protocol`. The short version:
  - **Protocol contract (always):** an old client parses messages from a new daemon, and a new daemon parses messages from an old client. New fields are optional; never narrow, never remove, never require. Wire schemas stay pure — no `.transform()`, `.catch()`, or `.preprocess()`.
  - **Feature contract (per-feature):** gate the capability once on `server_info.features.*`, then run the feature or tell the user to update the host. No fallback paths, no defensive branches.
  - **Every shim is tagged.** `// COMPAT(name): added in vX, remove after <date>` at the site that has to be deleted. `rg "COMPAT\("` is the cleanup backlog; untagged back-compat is permanent by accident.
  - **New RPCs use dotted namespaces with direction suffixes.** Follow [docs/rpc-namespacing.md](docs/rpc-namespacing.md): `domain.provider.operation.request` pairs with `domain.provider.operation.response`. Existing flat RPC names will migrate over time; don't add new ones.

## Platform gating

The app runs on iOS, Android, web (browser), and web (Electron desktop). Code is cross-platform by default. Gate only when you must. Import gates from `@/constants/platform`.

### The four gates

| Gate                       | Type      | When to use                                                                                                                 |
| -------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------- |
| `isWeb`                    | constant  | DOM APIs — `document`, `window`, `<div>`, `addEventListener`, `ResizeObserver`. This is the **exception**, not the default. |
| `isNative`                 | constant  | Native-only APIs — Haptics, `StatusBar.currentHeight`, push tokens, camera/scanner, `expo-av`.                              |
| `getIsElectron()`          | cached fn | Desktop wrapper features — file dialogs, titlebar drag region, daemon management, app updates, dock badges.                 |
| `useIsCompactFormFactor()` | hook      | Layout decisions — sidebar overlay vs pinned, modal vs full screen, single-panel vs split. From `@/constants/layout`.       |

### Decision matrix

| I need to...                                                   | Use                                                                       |
| -------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Access DOM (`document`, `window`, `<div>`, `addEventListener`) | `if (isWeb)`                                                              |
| Use a native-only API (Haptics, push tokens, camera)           | `if (isNative)`                                                           |
| Use an Electron bridge (file dialog, titlebar, updates)        | `if (getIsElectron())`                                                    |
| Switch layout between phone and tablet/desktop                 | `useIsCompactFormFactor()`                                                |
| Show something on hover, always-visible on native              | `isHovered \|\| isNative \|\| isCompact` (hover only works on web)        |
| Gate to iOS or Android specifically                            | `Platform.OS === "ios"` / `Platform.OS === "android"` (rare, keep inline) |

### Rules

- **Default is cross-platform.** Don't gate unless you have a specific reason.
- **Prefer Metro file extensions over `if` statements.** When a module has fundamentally different implementations per platform, use `.web.ts` / `.native.ts` file extensions instead of runtime `if (isWeb)` branches. Metro resolves the correct file at build time — the unused platform code is never bundled. Reserve `if (isWeb)` for small, inline checks (a single line or a few props). If you find yourself writing a large `if (isWeb) { ... } else { ... }` block, split into separate files instead.
  ```
  hooks/
    use-audio-recorder.web.ts    ← uses Web Audio API
    use-audio-recorder.native.ts ← uses expo-audio
  ```
  Import as `@/hooks/use-audio-recorder` — Metro picks the right file automatically.
- **Use `.electron.ts` / `.electron.tsx` for Electron-only web modules.** Electron is still the Metro `web` platform, but desktop dev/build sets `PASEO_WEB_PLATFORM=electron`, so Metro first looks for `.electron.*` files and falls back to normal `.web.*` files. Use this when the implementation depends on Electron-only behavior such as `webviewTag`, desktop preload APIs, or the Electron bridge. Keep plain browser web in `.web.*`, and keep native fallbacks in the base file or `.native.*`.
  ```
  desktop/browser/pane/
    index.electron.tsx ← Electron <webview> implementation
    index.web.tsx      ← plain web fallback
    index.tsx          ← native fallback
  ```
  Import as `@/desktop/browser/pane` — Electron desktop gets the `.electron.tsx` file, browser web gets `.web.tsx`, and native gets the native/base implementation.
- **NEVER use raw DOM APIs without `isWeb` guard.** DOM APIs crash native. Casting a RN ref to `HTMLElement` is a red flag — ensure the block is web-only.
- **NEVER use `onPointerEnter`/`onPointerLeave`.** They don't fire on native iOS.
- **Hover only works on web.** React Native's `onHoverIn`/`onHoverOut` on `Pressable` does NOT fire on native iOS/iPad — the underlying W3C pointer events are behind disabled experimental flags. For hover-to-show UI (kebab menus, action buttons), use `isHovered || isNative || isCompact` so the controls are always visible on native and hover-to-show on web.
- **Don't use Platform.OS as a proxy for layout capabilities.** Use breakpoints for layout decisions, not platform checks.
- **Import `isWeb`/`isNative` from `@/constants/platform`.** Never write `const isWeb = Platform.OS === "web"` locally.

## Debugging

Find the complete daemon logs and traces in the $PASEO_HOME/daemon.log

<!-- gitnexus:start -->

# GitNexus — Code Intelligence

This project is indexed by GitNexus as **bachuc** (125700 symbols, 460986 relationships, 1270 execution flows).

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact before editing.** Use `impact({target: "symbolName", direction: "upstream"})` or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .`; report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
- MUST warn on HIGH/CRITICAL `risk` pre-edit; never use `riskSharedAxes` to waive a HIGH/CRITICAL `risk` warning. Compare File/symbol: MCP File omits axes; Graph-RAG expands File.
- **MUST treat `risk: UNKNOWN` as unresolved, not as low.** An empty caller set is not evidence the symbol is unused — it can also mean the callers are not resolvable by the index (plain-object property access, dynamic dispatch, cross-language calls). `impact` pairs `UNKNOWN` with a `riskNote` saying so. Confirm with a text search before treating the symbol as safe to change or delete; do not proceed on the strength of a zero.
- **MUST use `query({search_query: "concept"})` for concepts/flows, `context({name: "symbolName"})` for a named symbol, or `impact` for blast radius, on read-only callers, dependencies, imports, or execution flow.** Graph first; text search only for empty/`UNKNOWN`/literals.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method before MCP/CLI impact analysis.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis, and never read `UNKNOWN` as an all-clear — it means the walk could not answer, which is the one verdict that requires confirming by other means.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit before MCP/CLI graph change analysis.

## Resources

| Resource                                | Use for                                  |
| --------------------------------------- | ---------------------------------------- |
| `gitnexus://repo/bachuc/context`        | Codebase overview, check index freshness |
| `gitnexus://repo/bachuc/clusters`       | All functional areas                     |
| `gitnexus://repo/bachuc/processes`      | All execution flows                      |
| `gitnexus://repo/bachuc/process/{name}` | Step-by-step execution trace             |

## CLI

| Task                                         | Read this skill file                               |
| -------------------------------------------- | -------------------------------------------------- |
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus-exploring/SKILL.md`       |
| Blast radius / "What breaks if I change X?"  | `.claude/skills/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?"             | `.claude/skills/gitnexus-debugging/SKILL.md`       |
| Rename / extract / split / refactor          | `.claude/skills/gitnexus-refactoring/SKILL.md`     |
| Tools, resources, schema reference           | `.claude/skills/gitnexus-guide/SKILL.md`           |
| Index, status, clean, wiki CLI commands      | `.claude/skills/gitnexus-cli/SKILL.md`             |

<!-- gitnexus:end -->
