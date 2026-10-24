# TASK-301 Outcome

## Status

DONE. Verdict: GO.

## Changed

- `packages/desktop-gpui/crates/diagram/assets/pele/pele.global.js` (new, vendored from `pele@1.10.0`, 709352 bytes)
- `packages/desktop-gpui/crates/diagram/assets/pele/LICENSE` (new, vendored)
- `packages/desktop-gpui/crates/diagram/Cargo.toml` (`rquickjs = { version = "0.14", default-features = false }`)
- `packages/desktop-gpui/crates/diagram/examples/pele_bench.rs` (new)
- `packages/desktop-gpui/Cargo.lock` (+40 lines: `rquickjs`, `rquickjs-core`, `rquickjs-sys`, one new `hashbrown` version)

## Contract

- Produces: `packages/desktop-gpui/crates/diagram/assets/pele/pele.global.js`, Pele 1.10.0 global build.
- Produces: `rquickjs = "0.14"` in `crates/diagram/Cargo.toml`. It uses `default-features = false`; no feature is needed to eval a script and call `pele.render`.
- Produces: init ms, overall p95 ms, and verdict below.
- The bench passes the source through `globalThis.__src` and evals `pele.render(__src).svg`, so each render includes the context entry and the string copy.

## Verified

- Baseline `./cargo.sh test -p bachuc-diagram` before any change: 9 passed, 0 failed.
- `head -c 70` of `pele.global.js`: `/*! Pele 1.10.0 | MIT License | https://github.com/obsidianmd/pele */`. The package came from `npm pack pele@1.10.0 --ignore-scripts` in `/tmp/pele-pack.MfVpeE`. Only the two files were copied. No package script ran.
- `CARGO_BUILD_JOBS=4 ./cargo.sh run --release -p bachuc-diagram --example pele_bench` ran 3 times (run 1 and 3 below, run 2 in `/tmp/pele_bench_run2.txt`). Exit 0 each time, `GO` each time.
- After adding `rquickjs`: `./cargo.sh test -p bachuc-diagram` -> 9 passed, 0 failed.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0. One `rquickjs` lock entry.
- `./cargo.sh fmt --check -p bachuc-diagram` -> exit 0 (after `fmt -p bachuc-diagram`).
- `./cargo.sh clippy -p bachuc-diagram --no-deps --example pele_bench -- -D warnings` -> clean.
- UI checker: none for this TASK.

### Bench output, run 1 (first run, quiet machine)

```text
init: 126.3 ms (runtime + context + load Pele)
sample            p50 ms    p95 ms svg bytes
flowchart           1.64      2.60      3726
sequence            0.60      0.97      3302
class               0.63      1.01      2196
state               1.41      2.16      3548
er                  0.75      1.03      3308
gantt               0.63      1.10      3340
pie                 0.23      0.39      2297
xychart             0.45      0.59      2909
mindmap             0.29      0.42      2656
flowchart-40        8.48     11.22     18734
overall p95: 8.48 ms over 500 renders
GO
```

### Bench output, run 2 (load average about 16)

```text
init: 117.5 ms (runtime + context + load Pele)
sample            p50 ms    p95 ms svg bytes
flowchart           2.25      5.52      3726
sequence            0.75      5.19      3302
class               0.78      2.80      2196
state               2.15      4.90      3548
er                  1.00      1.89      3308
gantt               0.84      1.94      3340
pie                 0.42      1.34      2297
xychart             0.53      0.83      2909
mindmap             0.37      3.51      2656
flowchart-40        9.49     22.63     18734
overall p95: 9.58 ms over 500 renders
GO
```

### Bench output, run 3 (final code, load average about 16)

```text
init: 141.5 ms (runtime + context + load Pele)
sample            p50 ms    p95 ms svg bytes
flowchart           1.84      5.50      3726
sequence            0.68      1.45      3302
class               0.81      1.28      2196
state               1.60      2.48      3548
er                  0.97      1.18      3308
gantt               0.73      1.17      3340
pie                 0.29      0.64      2297
xychart             0.51      0.72      2909
mindmap             0.27      0.46      2656
flowchart-40        7.44     11.70     18734
overall p95: 7.44 ms over 500 renders
GO
```

## Anti-goal

`bachuc-diagram` tests keep passing with no test changed. Checkpoints (command `./cargo.sh test -p bachuc-diagram`, source: cargo output, 2026-10-25):

- Before changes: 9 passed, 0 failed.
- After adding `rquickjs` and the bench: 9 passed, 0 failed.
- At completion: 9 passed, 0 failed (run after the dependency, bench, and `fmt`).

## Lessons

- Pele 1.10.0 on rquickjs 0.14 (QuickJS) in release: init 117 to 142 ms (limit 300), overall p95 7.4 to 9.6 ms (limit 16). All 10 samples render, with all 10 SVGs non-empty.
- The 40-node flowchart dominates the p95. Its own p95 was 11.2 to 22.6 ms, and its p50 7.4 to 9.5 ms. Under load average 16 its single-sample p95 crossed 16 ms, but the overall p95 did not. Small samples run at 0.3 to 2.5 ms p50.
- The overall p95 with 10 samples is set by the 40-node flowchart, so a larger real diagram can cross 16 ms. TASK-303 should render off the UI thread.
- Mermaid syntax that Pele accepted unchanged: `xychart-beta` with `bar` and `line`, `mindmap`, `gantt`, `pie title`, `erDiagram`, `classDiagram`, `stateDiagram-v2`.
- `rquickjs` needs no feature flag to eval a script and read a `String`. `CatchResultExt::catch(&ctx)` gives the JS exception text.
- `cargo fmt --check` fails on a new example until `fmt -p bachuc-diagram` runs. The first release build of the example took about 5 minutes.

Lead re-check (2026-10-25): `cargo.sh test -p bachuc-diagram` -> 9 passed; `fmt --check` -> ok; `metadata` -> ok; Pele header line matches. Bench output accepted from the builder (release run, heavy).
