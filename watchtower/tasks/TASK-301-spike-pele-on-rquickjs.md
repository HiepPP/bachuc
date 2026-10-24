# TASK-301 Spike Pele on rquickjs

Group: A (writes `crates/diagram/Cargo.toml`, shared with TASK-303 and TASK-304)
Class: risky

## Brief

Goal: Measure Pele on QuickJS through `rquickjs` and decide go or no-go for the plan.

Change: no QuickJS in the repo -> a vendored Pele, an `rquickjs` dependency, and a bench example that prints the timings.

Boundaries:

- Go when the warm render p95 is 16 ms or less, and runtime creation plus Pele load is 300 ms or less. See `## Decisions` in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- If the result is no-go, write the numbers in the outcome sidecar, set this TASK to `DONE`, and set TASK-302, TASK-303, and TASK-304 to `BLOCKED` with the reason. Do not revert the vendored file; the owner decides.
- Measure in the release profile. QuickJS is C, and the dev profile builds it at `-O0`, so dev numbers are not valid.
- The bench changes no app behavior. Do not touch `src/lib.rs` or `src/page.rs`.
- Trap: `pele.global.js` from npm: download it into a new empty folder, check its first line is `/*! Pele 1.10.0 | MIT License`, then copy only that file and the package `LICENSE`. Do not run any script from the package.
- Planner call: put the bench at `examples/pele_bench.rs`, so it builds against the crate's real dependencies.

How:

- Vendor `dist/pele.global.js` and `LICENSE` from `pele@1.10.0` into `crates/diagram/assets/pele/`.
- Add `rquickjs = "0.14"` to `[dependencies]`, with the fewest features that run a script and call a function.
- In `pele_bench`, time the creation of `Runtime` and `Context` plus evaluation of `include_str!("../assets/pele/pele.global.js")`. Then render each sample 50 times through `pele.render(src).svg` in the same context.
- Print init ms, p50 and p95 per sample, the overall p95, and `GO` or `NO-GO`. Exit non-zero on `NO-GO` or on any render error.

Files:

- [packages/desktop-gpui/crates/diagram/assets/pele/pele.global.js](packages/desktop-gpui/crates/diagram/assets/pele/pele.global.js) (new, vendored)
- [packages/desktop-gpui/crates/diagram/assets/pele/LICENSE](packages/desktop-gpui/crates/diagram/assets/pele/LICENSE) (new, vendored)
- [packages/desktop-gpui/crates/diagram/Cargo.toml](packages/desktop-gpui/crates/diagram/Cargo.toml) (add `rquickjs`)
- [packages/desktop-gpui/crates/diagram/examples/pele_bench.rs](packages/desktop-gpui/crates/diagram/examples/pele_bench.rs) (new)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (new `rquickjs` entries)

Interfaces:

- Produces: file `packages/desktop-gpui/crates/diagram/assets/pele/pele.global.js` (Pele 1.10.0 global build, defines `var pele`) for TASK-303
- Produces: dependency `rquickjs = "0.14"` in `crates/diagram/Cargo.toml` for TASK-303
- Produces: the measured init ms, overall p95 ms, and the `GO` or `NO-GO` verdict in `watchtower/tasks/TASK-301-outcome.md` for TASK-302

Expected result:

- WHEN `pele_bench` runs in the release profile THE bench SHALL print the init time, p50 and p95 per sample, the overall p95, and a verdict.
- WHEN every sample renders and both limits hold THE bench SHALL print `GO` and exit 0.
- IF a sample fails to render or a limit is crossed THEN THE bench SHALL print `NO-GO` with the reason and exit non-zero.

Samples (one each): flowchart with a decision and a cylinder node, sequence, class, state, ER, gantt, pie, xychart with bar and line, mindmap, and a flowchart with 40 nodes.

Anti-goal: the existing `bachuc-diagram` tests keep passing with no test changed; tripwire; read before changes, after adding `rquickjs`, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-diagram` -> passes before and after, same test count.
- `head -c 70 packages/desktop-gpui/crates/diagram/assets/pele/pele.global.js` -> starts with `/*! Pele 1.10.0 | MIT License`.
- `cd packages/desktop-gpui && ./cargo.sh run --release -p bachuc-diagram --example pele_bench` -> prints init ms, per-sample p50 and p95, overall p95, and `GO` or `NO-GO`. Copy the full output into the outcome sidecar.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exits 0, no duplicate lock entry.
