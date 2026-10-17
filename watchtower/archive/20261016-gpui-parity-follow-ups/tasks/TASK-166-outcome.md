# TASK-166 outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/vendor/gpui-base` (new): a `cp -R` copy of gpui-base 0.7.3 from the cargo registry, with license, `Cargo.toml`, and `Cargo.lock` kept.
- `packages/desktop-gpui/vendor/gpui-base/src/input/base/state.rs`: the `input-state` element gets `.role(gpui::Role::Group)`, plus a local `use gpui::StatefulInteractiveElement as _;` and a short comment in the same hunk.
- `packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md` (new): upstream version, the one change, and how to re-apply it.
- `packages/desktop-gpui/Cargo.toml`: `[patch.crates-io] gpui-base = { path = "vendor/gpui-base" }`. The `gpui-kit = "=0.7.3"` pin is unchanged.
- `packages/desktop-gpui/Cargo.lock`: cargo removed the registry `source` and `checksum` lines of gpui-base (2 lines).

## Contract

- Produces: `packages/desktop-gpui/vendor/gpui-base` patched through `[patch.crates-io]`, used by every crate; no Rust API change. gpui-base is not a workspace member (checked with `cargo metadata`), so `--workspace` does not build its tests.
- Role choice: GPUI filters `Role::GenericContainer` out of the AX tree, and `role()` debug-asserts against it (`gpui-pre-0.3.9/src/elements/div.rs:1310`). `Role::Group` is the most neutral role that makes a node. No label.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh tree -p bachuc-app -i gpui-base` -> `gpui-base v0.7.3 (/Users/hiep/Projects/bachuc/packages/desktop-gpui/vendor/gpui-base)`.
- `grep -n 'id("input-state")' -A3 packages/desktop-gpui/vendor/gpui-base/src/input/base/state.rs` -> line 4627 `.id("input-state")`, line 4628 `.role(gpui::Role::Group)`.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh check -p gpui-base` -> Finished, no errors.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test --workspace` -> first run failed with E0308 in `crates/client/src/core.rs:1438` (group A mid-edit, not this TASK). Retry after about 4.5 minutes: exit 0, 345 passed, 0 failed, 5 ignored.
- The "has no accessibility node" log is gone at runtime -> UNVERIFIED (unattended): code tests only.

## Anti-goal

- Checkpoint: `diff -r ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.7.3 packages/desktop-gpui/vendor/gpui-base`; value: "Only in vendor/gpui-base: BACHUC-PATCH.md" and one `state.rs` change (added lines 4622-4625 and 4628); source: `diff -r`; time: 2026-10-16 15:52.

## Lessons

- The spec Change line and ADR-0044 say "a role and a label", but the Boundaries planner call says "with no label". The build followed the Boundaries line: the outer frame already carries `Role::TextInput`, so a second label would duplicate it in the AX tree.
- `StatefulInteractiveElement::role` needs the trait in scope. A block-local `use` keeps the patch in one hunk.
