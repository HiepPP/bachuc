# ADR-0066 Standalone Rust Decider probe crate over a second backend in jev-evaluator

Status: accepted
Date: 2026-10-23
Supersedes: -
Scope: `packages/desktop-gpui/crates/decider-probe/**`, `hiep-plugins/plugins/jev-evaluator/server/**`

## Context

- Decider for Mac serves `pplx-decider-flash-v1.2` at `http://127.0.0.1:8080/v1/decisions`. A live request on 2026-10-23 accepted `noul`, `choice`, and `score`, and rejected `boolean`.
- All Jev callers share [hiep-plugins/plugins/jev-evaluator/server/typesafe.mjs](hiep-plugins/plugins/jev-evaluator/server/typesafe.mjs). A Decider backend there would reuse the gate and the benchmark at once.
- The later integration target is a `DecisionService` in the GPUI client, written in Rust.
- The regex tier `tier0` lives in TypeScript in [hiep-plugins/plugins/jev-permission-gate/server/policy.ts](hiep-plugins/plugins/jev-permission-gate/server/policy.ts).

## Decision

- The probe is a standalone Rust crate, `decider-probe`, in the GPUI Cargo workspace. It does not depend on `gpui-kit`.
- Node scripts beside the crate extract events with `tier0` and call Jev through `evaluateWithTypesafe`. The regex is not ported to Rust.
- Hiep chose this on 2026-10-23 in the watchtower interview for TASK-298 to TASK-300.

## Consequences

- The Decider HTTP client and the metrics code are in Rust and can move into a later `DecisionService`.
- The plugins do not gain a Decider backend from this work. A plugin switch to Decider needs its own TASK.
- The probe needs two toolchains: `./cargo.sh` and Node with the gate's `tsx`.

## Options Considered

- Add Decider as a second backend in `jev-evaluator`: rejected. Hiep wants the probe code near the Rust integration target.
- Port `tier0` to Rust: rejected. Two copies of the regex can drift.

## Revisit If

- The probe meets the go criteria and the first integration is a plugin, not the GPUI client.
- `jev-permission-gate` needs a local backend before a GPUI `DecisionService` exists.
