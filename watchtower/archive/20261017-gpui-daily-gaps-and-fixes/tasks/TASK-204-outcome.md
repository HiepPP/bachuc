# TASK-204 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/model.rs`: new types `MergeMethod`, `GitAction`, `GitActionReply`, `GitStatus`, `PrMergeCapability`, `PrCheck`, `PrStatus`, `PrStatusReply`, `PrTimelineItem`, `StashEntry`, `WorkspaceScript`, `GitStatusEvent`, each with its parser.
- `packages/desktop-gpui/crates/client/src/core.rs`: new `Pending` replies, the request methods, `git_response`, `checkout_status_update` and `script_status_update` in `SESSION_EVENTS` and in `message`, workspace `scripts` kept per workspace, and `take_git_status_events`.
- `packages/desktop-gpui/crates/client/src/lib.rs`: the exports, `impl EventEmitter<GitStatusEvent> for Client`, `publish` emits the new events, and one wrapper per call.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: 15 new tests.

## Contract

- Every "Produces:" line is implemented with its exact name and signature. `ClientCore` has the same methods without `cx`, and `ClientCore::take_git_status_events()` for tests.
- `git_action`: `Commit` sends `addAll: true`. `PullAndPush` sends the push only after the pull succeeds. `CreatePr` sends no title or body. `MergeBranch` sends `strategy: "merge"` and `requireCleanTarget: true`. `MergeFromBase` sends `requireCleanTarget: true`. Neither merge sends a `baseRef`, so the daemon uses the checkout base (`checkout-session.ts:799`).
- `SetAutoMerge` uses `checkout.forge.set_auto_merge.request` when `server_info.features.checkoutForgeSetAutoMerge` is true. Otherwise it uses the GitHub request when `checkoutGithubSetAutoMerge` is true; that path is tagged `COMPAT(githubAutoMergeRpc)`. If the host has neither feature, the call fails with "Update the host to use auto-merge actions.", the same text as Electron.
- Errors: a checkout `{code, message}` error and an `rpc_error` both give `Err(message)`. A timeline `{kind, message}` error gives the message, or the kind when the message is empty. The script start `error` string gives `Err`.
- `git_status`: a folder outside git, or a `NOT_GIT_REPO` error, gives `GitStatus::default()` (`is_git: false`). A null `aheadBehind` or null origin count gives 0. `has_upstream` is true when `aheadOfOrigin` is a number, which matches Electron `policy.ts:525-526`.
- `PrMergeCapability` reads only the `github` block. A PR without that block, such as one from another forge, gets `PrMergeCapability::default()`. `auto_merge_method` maps GitHub's upper-case names.
- `PrTimelineItem`: an empty `url` becomes `None`. An item of an unknown kind reads as a `Comment`.
- `GitStatusEvent::Changed { cwd }` comes from `checkout_status_update`. `ScriptsChanged { workspace_id }` comes from `script_status_update`, and also from a workspace descriptor whose `scripts` changed.

## Verified

- Baseline `./cargo.sh test -p bachuc-client` took 10.95 s wall. It passed.
- `./cargo.sh test -p bachuc-client` passed, with 39 tests in rpcs. The warm wall time was 7.19 s (66% of baseline). Test execution went from 5.91 s to 6.11 s (+3%). The anti-goal holds.
- `./cargo.sh test -p bachuc-client git_status` passed. The non-git and null `aheadBehind` cases pass.
- `./cargo.sh build -p bachuc-client --examples` exited 0. `./cargo.sh build -p bachuc-app --examples` exited 0. `./cargo.sh build -p bachuc-app` exited 0.
- `./cargo.sh fmt --check -p bachuc-client` exited 0.
- `./cargo.sh metadata --format-version 1` exited 0.
- `clippy -p bachuc-client --tests` gave no client warnings. The 2 warnings it gave come from the existing `bachuc-protocol` build script.
- No UI checks apply to this TASK.

## Lessons

- The generated protocol types encode a `z.number()` field as f64, so the client sends `prNumber` as `9.0`. Zod reads it as 9. In a test, compare it with `as_f64()`.
- The fake daemon harness does not collect `GitStatusEvent`. Call `harness.core.take_git_status_events()` inside `pump_until` instead.
- `GitAction::MergeBranch` and `MergeFromBase` carry no base ref. If TASK-210 needs a base other than the checkout base, it needs a new field.
