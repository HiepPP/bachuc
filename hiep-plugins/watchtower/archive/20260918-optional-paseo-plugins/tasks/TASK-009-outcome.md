# TASK-009 Outcome

## Outcome

Status: DONE

Changed:
- Added bounded Node/package-manager discovery, category overrides and provenance to the shared preflight engine.
- Added workspace-bound read-only MCP injection for new built-in Codex/Claude agents; existing entries remain intact.
- Added typed optional Jev handoff, offline fixtures, explicit synthetic live runner and integration documentation.
- Fixed fresh Yarn Classic PnP detection found during independent review; added the failing fixture before the fix.

Contract:
- Discovery never executes scripts, version files, installs or repairs. Ambiguous layouts and coverage remain unknown.
- MCP accepts only an empty object. Host-controlled canonical cwd binds the instance; arbitrary directory overrides fail.
- Runtime probes retain daemon PATH semantics; the injected MCP captures that PATH. Workspace-owned executables are not probed.
- Explicit categories replace discovery; empty lists disable checks. Malformed settings block without fallback.
- Jev never changes measurements, grants permission or runs repairs. Unavailable/error/invalid evaluation has no proceed fallback.

Verified:
- `npm run format && npm run typecheck && npm run lint && npm test` in the preflight directory passed; 16 offline tests, zero lint warnings.
- Offline checks cover ranges/prereleases/conflicts, missing metadata, npm/pnpm/Yarn/PnP/monorepos, overrides, metadata limits, symlinks, nonexecution, MCP list/call/invalid input/unavailable workspace, hook coexistence and evaluator errors/timeouts.
- Daemon, native app and SDK verified at 0.8.0. Public plugin reference and installed lifecycle declarations checked; host source binds selected workspace cwd before hooks.
- Final plugin reload passed. Preflight and Jev both running with no load errors; main daemon not restarted.
- Disposable Paseo Codex agent `75466df0-23d5-4b4c-85a6-c6f88b2d7116` received and called both injected tools. Host runtimeInfo independently confirmed gpt-6-astra, low effort and auto-review mode; archived/closed state verified.
- Three frozen synthetic scenarios each called real Jev once: docs -> proceed (699/46/745 tokens); login-down -> prepare_environment (719/46/765); login-unknown -> need_more_evidence (715/49/764). Total 2,274 tokens; no API errors or retries.
- Inputs, raw preflight evidence, full Jev results and usage retained in [ignored acceptance artifact](../../../../plugins/workspace-preflight/artifacts/task009-agent-results.json). Expected choices stored separately and not sent to Jev.
- Parent replayed recorded inputs/results through final typed handoff offline: exact fixture equality, 3/3 valid expected choices, measurements unchanged. No additional API calls.
- Native `/preflight` and refresh verified on workspace `wks_4e9ca2ea5a743654`, with discovery/explicit reports at roughly 320px and 650px panel widths in a 1300x768 window. Text wrapped and unknown coverage remained visible.
- Jev temporarily disabled for standalone panel acceptance: explicit refresh and discovery refresh worked; Jev then restored to running.
- Runtime PATH adjustment after UI acceptance passed offline and stdio tests and final host reload; layout/rendering unchanged. An additional native runtime refresh was interrupted by user navigation and was not counted as new evidence.
- Temporary `.paseo/preflight.json` and its task-created directory removed. Panel width restored; newer user navigation left untouched. Disposable agent archived. No persistent fixture processes.
- `git diff --check` passed. Source snapshot SHA-256: `bb541f22067e336c0d61d9e4ba01776c38ff068303f9b3d5e0695f8674943df1`; file inventory saved under ignored artifacts.

Limits:
- Live Jev scenarios are synthetic acceptance examples, not production task evaluation or model calibration.
- Shell runtime/environment may differ from daemon PATH. Supported root layouts only; unsupported layouts require explicit categories.
- Durable contracts are in plugin READMEs; no separate Watchtower memory file was needed. No commit or push.
