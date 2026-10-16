# TASK-166 Vendored gpui-base with an input role

Group: J (writes `packages/desktop-gpui/Cargo.toml` and `packages/desktop-gpui/vendor/gpui-base`)
Class: risky

## Brief

Goal: Stop the "has no accessibility node" log for every focused text input (Q-087), per [ADR-0044](watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md).

Change: gpui-base 0.7.3 `src/input/base/state.rs` lines 4622-4625 track focus on `div().id("input-state")` with no role -> a vendored copy with `.role(..)` and `.aria_label(..)` on that element.

Boundaries:

- Copy the published crate from `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.7.3` into `packages/desktop-gpui/vendor/gpui-base`, keep its license files, and add `[patch.crates-io] gpui-base = { path = "vendor/gpui-base" }` to the workspace `Cargo.toml`.
- Change only the `input-state` element. The outer frame already has `Role::TextInput` (`src/input/base/mod.rs:236`), so give `input-state` a role that adds no second text field and no second label to the AX tree. Planner call: the most neutral container role GPUI offers (check the `Role` enum used by `div().role()`), with no label.
- Add a `vendor/gpui-base/BACHUC-PATCH.md` with the upstream version, the one change, and how to re-apply it after a gpui-kit upgrade.
- Keep `gpui-kit = "=0.7.3"` pins unchanged.

Files:

- [packages/desktop-gpui/Cargo.toml](packages/desktop-gpui/Cargo.toml)
- `packages/desktop-gpui/vendor/gpui-base` (new)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated)

Interfaces:

- Produces: `packages/desktop-gpui/vendor/gpui-base` patched through `[patch.crates-io]`, used by every crate; no Rust API change, for TASK-167

Expected result:

- The workspace builds and tests with the vendored crate, and `cargo tree` shows gpui-base from the vendor path.

Anti-goal: the vendored copy differs from upstream only in the `input-state` lines and the new `BACHUC-PATCH.md`; tripwire; read with `diff -r` at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh tree -p bachuc-app -i gpui-base` -> shows the `vendor/gpui-base` path
- `grep -n 'id("input-state")' -A3 packages/desktop-gpui/vendor/gpui-base/src/input/base/state.rs` -> a `.role(` line follows the id
- Anti-goal: `diff -r ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.7.3 packages/desktop-gpui/vendor/gpui-base` -> only the `state.rs` hunk and `BACHUC-PATCH.md`
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test --workspace` -> pass
