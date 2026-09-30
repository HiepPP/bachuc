# TASK-009 Task-aware preflight with Jev

Group: A (shares the preflight plugin and root catalog)
Class: risky

## Brief

Goal: Help an agent decide whether the workspace is ready for its current task, with minimal repository configuration.

Change: Detect basic prerequisites, expose preflight through MCP, and let the agent ask Jev for the next step.

How:

- Use the paseo-plugin skill. Verify the installed daemon, client, SDK, and MCP launch contracts before implementation.
- Reuse TASK-001 checks and the existing Jev evaluator. Preserve both plugins as independent installations.
- Read only bounded metadata from the exact workspace: root package.json, .nvmrc, .node-version, and recognized package-manager lockfiles.
- Infer Node requirements from declared version constraints. Use correct version-range matching, not a guessed major version.
- Record conflicts, unsupported aliases, missing declarations, and unsupported repository layouts as unknown with source evidence.
- Infer node_modules presence only for supported node_modules-based layouts. Do not call missing dependencies for Yarn PnP layouts.
- Treat multiple lockfiles or unclear package-manager declarations as ambiguous. Do not recommend an arbitrary installation command.
- Do not execute repository scripts, version files, install commands, or lifecycle hooks during discovery.
- Do not infer health URLs or ports from arbitrary script text. These remain explicit settings or unknown prerequisites.
- Use explicit preflight settings before inferred settings. A present category replaces its inferred category; an empty list disables it.
- Missing settings enable discovery. Invalid explicit settings remain a blocker rather than silently falling back.
- Return bounded structured evidence with check IDs, category, source, observation, status, and timestamp.
- Keep raw pass, blocker, and unknown results unchanged after evaluation. All checks passing does not prove task coverage.
- Add a read-only MCP tool named workspace_preflight that calls the same check engine as the UI.
- Bind the MCP instance to its creating workspace through verified Paseo context. Reject arbitrary directory overrides from tool input.
- Inject the MCP entry into new built-in Codex and Claude agents without overwriting an existing same-name entry.
- Preserve Jev injection and credentials handling. Existing agents and provider-internal subagents must have documented limitations.
- Document an agent workflow: collect task requirements, call workspace_preflight, then call jev_evaluate with sanitized evidence.
- Ask Jev a typed choice question: proceed, prepare_environment, or need_more_evidence.
- Give Jev the task's required checks and coverage gaps. Missing task-critical evidence must remain explicit.
- Jev judges relevance and the next step; it cannot rewrite measurements, approve permissions, or execute repairs.
- A missing Jev tool, timeout, invalid answer, or API error leaves evaluation unavailable. Preserve evidence and do not assume readiness.
- Send only the task summary and necessary evidence intended for external evaluation. Exclude credentials and unrelated repository contents.
- Keep `/preflight` and its report usable without Jev. Display or document the difference between discovery and explicit configuration.
- Document one reproducible invocation for the agent workflow. No automatic per-turn Jev calls or lifecycle repair loop.
- Add focused offline tests and a separate, explicit synthetic live verification path. Do not hide failures or retry model answers.

Files:

- [Preflight server entry](../../../../plugins/workspace-preflight/index.server.ts): Register the MCP injection alongside the existing RPC.
- [Preflight server modules](../../../../plugins/workspace-preflight/server): Reuse the engine; add bounded discovery and an MCP entry.
- [Preflight contracts](../../../../plugins/workspace-preflight/shared/preflight.ts): Add evidence provenance and version constraints without breaking explicit settings.
- [Preflight panel](../../../../plugins/workspace-preflight/client/panel.tsx): Show relevant discovery sources and unknown coverage.
- [Preflight package](../../../../plugins/workspace-preflight/package.json): Declare required MCP dependencies and focused verification scripts.
- [Preflight lockfile](../../../../plugins/workspace-preflight/package-lock.json): Regenerate through npm after dependency changes.
- [Preflight manifest](../../../../plugins/workspace-preflight/paseo-plugin.json): Declare verified host compatibility.
- [Preflight tests](../../../../plugins/workspace-preflight/tests): Cover discovery, precedence, MCP boundaries, and evaluation handoff.
- [Preflight README](../../../../plugins/workspace-preflight/README.md): Document discovery limits, overrides, and the agent-to-Jev workflow.
- [Jev README](../../../../plugins/jev-evaluator/README.md): Document the typed preflight evaluation example and failure behavior.
- [Root catalog](../../../../README.md): Describe the optional integration.

Expected result:

- A supported Node repository gets useful preflight evidence without a separate preflight configuration file.
- An agent can collect workspace evidence and ask Jev which step fits its current task.
- A docs-only task can proceed without a backend; browser login verification must address required backend failures.
- Unknown coverage stays visible. Jev failure does not become a pass, and repair commands never run automatically.

## Verify

- Run `npm run format && npm run typecheck && npm run lint && npm test` in the preflight plugin; all pass.
- Discovery fixtures cover supported version ranges, matching and wrong runtimes, missing declarations, and conflicting version files.
- Fixtures cover npm, pnpm, Yarn node_modules, Yarn PnP, multiple lockfiles, and unsupported monorepo layouts.
- Precedence fixtures prove explicit categories replace inferred ones, empty categories disable them, and malformed settings block.
- Fixtures prove metadata size limits, workspace confinement, and that scripts and repair suggestions never execute.
- MCP protocol tests exercise tools/list and tools/call, workspace binding, invalid input, and unavailable workspaces.
- New-agent hook tests preserve existing MCP entries, leave unsupported providers unchanged, and coexist with Jev injection.
- Offline handoff tests preserve raw evidence, carry task coverage, validate choice keys, and retain evaluator errors without a proceed fallback.
- Freeze expected choices before live verification: docs with an irrelevant backend failure -> proceed; login UI with required backend down -> prepare_environment; login UI with unknown backend health -> need_more_evidence.
- Reload preflight on the intended host. Require running with no load error; confirm Jev remains running.
- Create one disposable supported Paseo agent and verify it receives both MCP tools. Do not substitute a provider-internal subagent.
- Run the three synthetic scenarios through the real tools once each. Record inputs, observed choices, usage, and failures separately from offline evidence.
- The live Jev check consumes quota and is an explicit acceptance step, not part of routine npm tests.
- Inspect the final agent report: measurements stay unchanged, task relevance is separate, and no repair or permission approval executes.
- Verify `/preflight` still works without Jev, with both discovered and explicit settings, at wide and compact panel widths.
- Restore temporary configuration and UI state, stop fixture processes, and archive the disposable agent. Preserve unrelated WIP.

Prompt:

```text
Implement TASK-009 only using the paseo-plugin skill. Preserve unrelated work and existing plugin behavior. Do not commit or push.
```
