# ADR-0044 Vendored gpui-base patch over a git fork or accepting the log

Status: accepted
Date: 2026-10-16
Supersedes: -
Scope: `packages/desktop-gpui/Cargo.toml`, `packages/desktop-gpui/vendor/**`

## Context

- gpui-base 0.7.3 `src/input/base/state.rs` lines 4622-4625 track focus on `div().id("input-state")` with no role, so GPUI logs "has no accessibility node" for every focused text input (Q-087, previous plan's TASK-155 re-checks).
- No crate in the repo can add the role; the element lives in the registry crate behind `gpui-kit = "=0.7.3"`.
- The owner chose the fix in the follow-up planning interview on 2026-10-16.

## Decision

- Copy gpui-base 0.7.3 into `packages/desktop-gpui/vendor/gpui-base`, add a role and a label to `input-state`, and point every crate at it with `[patch.crates-io]`.

## Consequences

- Builds stay offline and reproducible from the repo.
- Each gpui-kit upgrade must re-apply the patch; `vendor/gpui-base/BACHUC-PATCH.md` records the change.
- The repo grows by the size of one crate's source.

## Options Considered

- A git fork of gpui-base in the owner's account: rejected. It needs a fork to maintain and network access at build time.
- Accept the info-level log: rejected. The owner wants the AX tree clean for VoiceOver.

## Revisit If

- gpui-base upstream adds a role to `input-state`.
- The client moves off gpui-kit 0.7.3.
