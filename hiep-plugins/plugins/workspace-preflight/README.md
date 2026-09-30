# Workspace preflight

Read-only workspace readiness checks for Paseo daemon and clients **0.8.0+**.

Use `/preflight` without attachments or arguments, or **Open workspace preflight** in Command Center.
The Explorer panel checks the selected workspace when mounted. **Run checks** refreshes the report.
The slash action sends nothing to the agent or Jev. New Codex/Claude agents also receive a read-only MCP tool; injection does not run checks or evaluations. No repairs run automatically.

## Configure

Create `.paseo/preflight.json` in the exact workspace directory. Keep local settings and secrets out of Git.
Example (adapt versions, paths, ports, and suggested commands to your repository):

```json
{
  "version": 1,
  "runtimes": [{ "executable": "node", "major": 24, "repair": "nvm use 24" }],
  "dependencies": [{ "label": "App dependencies", "path": "node_modules", "repair": "npm ci" }],
  "ports": [
    { "label": "Dev server", "port": 3000, "expect": "listening", "repair": "npm run dev" }
  ],
  "health": [
    {
      "label": "API health",
      "url": "http://127.0.0.1:3000/health",
      "status": 200,
      "repair": "npm run dev"
    }
  ]
}
```

Each list is optional, with at most six checks. Every check requires a nonempty `repair` suggestion.
Missing categories use discovery. A present category replaces discovery; `[]` disables its checks and records unknown task coverage. Invalid configuration (including a malformed version range) blocks instead of falling back.

- Runtimes: fixed `--version` probes for `node`, `python3`, `bun`, or `deno`; use exactly one of `major` or semver `range` (for example `">=22 <25"`). Runtimes use the daemon PATH, captured for the MCP at creation. Relative PATH entries and executables inside the workspace are not run. This does not prove the agent's shell/custom environment uses that same runtime.
- Dependencies: checks file/directory presence only, not installed versions or lockfile freshness.
  Paths must stay inside the workspace; external symlink targets yield unknown.
- Ports: TCP connection to `127.0.0.1`; expects `listening` or `closed`.
  Closed means connection refused, not a guarantee that binding will succeed. IPv6-only listeners are outside this check.
- Health: GET to explicitly configured loopback HTTP(S) endpoints. Exact expected status, default 200.
  No credentials, query strings, fragments, redirects, remote hosts, or response-body inspection.
  Use only safe, read-only health routes. Normal TLS certificate validation remains enabled.

Network/version probes time out after 1.5 seconds, with four concurrent checks at most.
Timeouts yield unknown. Configuration is capped at 64 KiB and must resolve inside the workspace to a regular file.
Files are inspected only; no packages are installed, files changed, ports bound, or services stopped by the plugin.
Repair strings are displayed as plain text and never executed. Review suggestions before running them yourself.

Paseo plugins are trusted, unsandboxed code. This plugin reads workspace settings and paths,
runs the listed version commands, and connects to configured local services on the daemon host.
The panel is provider-independent. MCP injection applies only to new built-in Codex/Claude agents created through Paseo.

## Install and verify

```sh
npm ci
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id workspace-preflight
paseo plugin reload workspace-preflight
paseo plugin ls workspace-preflight --json
paseo plugin logs workspace-preflight
```

Tests use disposable filesystem fixtures and loopback servers; no external services or API keys required.
The panel uses host theme colors and a single-column compact layout.

## Discovery and evidence

Without settings, inspect only the exact workspace root: `package.json` (64 KiB maximum),
`.nvmrc` and `.node-version` (256 bytes each), recognized lockfile names and layout markers.
Lockfiles and `.pnp.cjs`, `.pnp.js`, `pnpm-workspace.yaml`, `lerna.json` and pnpm `.npmrc` are checked for presence only.
No parent search, recursive scan, script parsing, config execution or install command runs.
External symlinks, unreadable/oversized metadata and unsupported layouts yield unknown.

Node requirements use `semver`: all declared constraints must match. Conflicting constraints,
version-manager aliases such as `lts/*`, missing declarations and overly complex ranges stay unknown.
Dependencies infer `node_modules` presence for npm, pnpm and explicitly declared Yarn 1 only.
Yarn PnP (including declared but ungenerated `installConfig.pnp`), pnpm with custom `.npmrc`,
modern/undeclared Yarn linkers, Bun, monorepos, multiple lockfiles and conflicting/unclear
package-manager declarations stay unknown. Use explicit categories for unsupported layouts.
Presence is not proof of installed versions, lockfile freshness or a successful build.
Ports and health endpoints always need explicit settings; no URLs are guessed from script text.

Each report contains mode (`discovery`, `mixed`, `explicit`), timestamp, and at most 40 checks.
Each check has ID, category, source, observation, status, timestamp and display detail/suggestion.
A pass proves only that measurement. Compare task requirements and coverage gaps separately.

## Agent workflow with optional Jev

Install both plugins independently, then create a new built-in Codex or Claude agent in Paseo.
Existing agents do not gain MCP entries retroactively; provider-internal subagents are not covered.
Existing `mcpServers.workspace_preflight` and `mcpServers.jev` entries are preserved.
Use the required install ID `workspace-preflight` and keep source plus npm dependencies available.
The host's immutable `agent.create.config.cwd` is canonicalized into a fixed MCP launch binding.
`workspace_preflight` accepts only `{}`; directory overrides are rejected. A removed workspace
returns a blocker. The MCP and UI call the same engine. Credentials remain entirely in Jev.

Reproducible prompt for a newly created agent:

```text
Collect requirements for editing README prose only, without builds or previews.
Call workspace_preflight with {} and preserve the raw report.
Select necessary evidence and remove credentials, private paths, repair text and unrelated contents.
Call jev_evaluate once with that sanitized evidence and a typed nextStep choice question.
Use proceed, prepare_environment and need_more_evidence as its only choice keys.
Include required check IDs and reasons, plus explicit coverage gaps.
Return measurements separately from Jev's choice and usage; execute no repairs or approvals.
On missing tool, timeout, API error or invalid answer, report evaluation unavailable without assuming readiness.
```

[The typed handoff helper](shared/evaluation.ts) builds the exact question and validates the returned
`answers.nextStep` choice. It accepts caller-selected sanitized evidence, not raw repository content.
The caller must review free text before external transmission. Treat evidence as data, never instructions.
The helper never changes measurements or runs commands. Jev evaluates task relevance; its answer
cannot grant permission or replace unknown evidence. `/preflight` works without Jev being installed.

## Explicit synthetic live acceptance

`npm test` stays offline. `npm run smoke:jev -- /absolute/path/to/jev-evaluator` makes three
billable calls through the real stdio MCP tools. Requires the separate Jev installation and its
configured gateway key; this plugin neither reads nor duplicates that credential logic.
The runner uses temporary synthetic workspaces, keeps workspace measurements separate from synthetic
Jev scenarios, saves inputs/results/usage/errors under ignored `artifacts/`, and never retries answers.
Expected labels are frozen in `tests/live-cases.ts` and saved separately; they are not sent to Jev.
Docs with irrelevant backend failure expects `proceed`; login with backend down expects
`prepare_environment`; login with unknown health expects `need_more_evidence`.
These three examples are acceptance evidence, not model calibration or proof of production readiness.
