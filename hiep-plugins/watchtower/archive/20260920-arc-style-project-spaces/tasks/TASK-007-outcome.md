# TASK-007 Outcome

## Outcome

Status: DONE

Changed: Scoped MCP broker and real Paseo children preserve saved profile settings, including explicit selections.

Verified: Profile preservation, invalid inputs, duplicate IDs, bridge scope and a real stdio MCP round trip pass. Live routed interval passed external checks using codex/gpt-5.6-luna/max; the desktop slash command opens its panel.

Shared checks: format, typecheck, lint and all 24 offline/integration tests pass on installed Paseo SDK 0.8.0. See [verification report](../../../../plugins/jev-orchestrator/VERIFICATION.md) for live evidence and failures.

Limitations: Existing agents need recreation to receive MCP injection. Populated panel and native mobile acceptance are not claimed.

Commit/push: none. Paseo application source is unchanged.

## Direct routing follow-up

Changed: Added an opt-in direct route with one Jev choice and one agent, without a parent LLM.

Contract: Jev selects a supported model and effort. Profile permission modes and features stay unchanged.
The user's new request permits per-task effort selection. Saved profile files are not changed.
Duplicate requests return the same agent. Interrupted creation never replays automatically.

Verified: Format, typecheck, lint, 43 tests, installed reload and eight real benchmark runs passed.
The desktop panel loads profiles and validates inputs. Live model/effort matches the selected pair.
See the [direct routing report](../../../../plugins/jev-orchestrator/VERIFICATION.md#direct-modeleffort-routing--2026-09-21).

Effectiveness: Integration works. The tested routing policy does not save tokens or time against the fixed baseline.
Commit/push: none.
