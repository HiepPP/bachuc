# TASK-210 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/git_actions.rs` (new): the pure policy, a port of `buildGitActions` in `policy.ts` and of the inputs that `use-actions.tsx` derives. It also holds the press rule, the action-to-`GitAction` step, the ship default per pick, the result texts, and the `RefreshGate` coalescer. It has 21 tests.
- `packages/desktop-gpui/crates/explorer/src/git_actions_ui.rs` (new): `GitActionsButton`, with the primary button, the caret menu, and a spinner while an action runs.
- `packages/desktop-gpui/crates/explorer/src/lib.rs`: `pub mod git_actions`, `mod git_actions_ui`, and the exports `ShipDefault`, `GitActionsButton`, and `GitActionsEvent`.

## Contract

- Produces: `GitActionsButton::new(client: Entity<Client>, cwd: String, cx: &mut Context<GitActionsButton>) -> GitActionsButton`.
- Produces: `ShipDefault { Pr, Merge }`. The default is `Pr`, as in Electron.
- Produces: `GitActionsEvent { Finished { ok, message }, OpenUrl { url }, ArchiveWorkspace, ShipDefaultChanged { cwd, default } }`, through `impl EventEmitter<GitActionsEvent> for GitActionsButton`.
- Produces: `GitActionsButton::set_ship_default(&mut self, default: ShipDefault, cx: &mut Context<GitActionsButton>)`.
- Each "Consumes:" line from TASK-204 is used by its exact name.
- `ShipDefaultChanged` is emitted only when a pick changes the value: Create PR, Merge PR, and Enable auto-merge give `Pr`, and Merge locally gives `Merge`.
- The button emits `Finished { ok: false, message }` when a user presses an item that has an unavailable message. Electron shows that message as an info toast.
- A failed action emits `Finished { ok: false }` with the daemon message. If the message is empty, the button uses Electron's fallback text, for example "Failed to push".
- Refresh: the button calls `git_status` and `pr_status` on creation, on `ClientEvent::Connected`, and after every action. A `GitStatusEvent::Changed` for its cwd starts one 500 ms timer, and more events before the timer fires add nothing. A generation counter drops older answers.
- All actions are disabled while the first status loads (no button is drawn yet), when the status request fails, outside git, and while any action runs. This follows the spec. Electron disables only the action that runs.
- If a merge succeeds (Merge PR or Merge locally), Archive becomes the primary action, as `setPostShipArchiveSuggested` does. When the merged PR or the archive flag promotes Archive, the button emits `ArchiveWorkspace`.

## Verified

- `./cargo.sh tree -p bachuc-explorer | rg bachuc-app` -> no match, before and after the change.
- `./cargo.sh test -p bachuc-explorer git_actions` -> 21 passed. This covers the dirty, behind-origin, ahead-of-origin, and mergeable-PR primary cases, plus the auto-merge, ship-default, update-from-base, archive, list-order, and message cases.
- The not-git test and the status-loading test (`actions_disabled`) both give every action disabled, and a press is ignored.
- `./cargo.sh test -p bachuc-explorer` -> 21 + 38 + 21 tests passed.
- `./cargo.sh build -p bachuc-explorer` -> exit 0.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `./cargo.sh clippy -p bachuc-explorer --tests` -> no warnings in the new files. The 3 warnings it gives already existed, in `file_view.rs:86` and the `bachuc-protocol` build script.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- Split button look, caret menu, spinner, and the button in the header: DEFERRED (UI checker). The app adds the button in TASK-202.

## Lessons

- `PrMergeCapability` from TASK-204 does not carry the GitHub merge queue flags (`isMergeQueueEnabled`, `isInMergeQueue`) or `viewerDefaultMergeMethod`. So the policy never treats a merge queue as a block. It also prefers the first allowed method in Electron's order: squash, merge, rebase.
- A PR without `github` facts parses to `PrMergeCapability::default()`. The policy reads that value as Electron's `null` capability, and then uses the mergeable and in-sync fallback.
- `has_upstream: false` stands for a null `aheadOfOrigin` and a null `behindOfOrigin`.
- In Electron the split button draws nothing when there is no primary action. TASK-202 must allow for a zero-size button when the folder is not git or the first status has not arrived.
- The view uses the forge words "GitHub" and "PR" only. GitLab "MR" wording is not ported.

## UI Checks

Run 2026-10-18 by the UI checker (run 3). Bundle `Bachuc GPUI Check` built from HEAD `2d5734c11` (includes `e9a78e843`, `77a9730cb`, and `22906f2cb`), PID 31420, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The app stayed in the background. The checker forced each frame with the AX "zoom the window" action and read the AX tree after a no-op F15 key. Scratch repo `/tmp/t202-repo` (branches `main` and `feature-x`, script `hello`) was the live workspace `wks_4a3662191ab77c60` with disposable mock agents "t202 scratch", "t200 archive me", "t200 board card", and "t208 cpu stream". The workspace was archived after the run; PID 31420 quit and exited.

- Split button in the header: PASS. "Commit" primary with a commit glyph and a separate caret, inside one bordered group (`/tmp/r3-16-changes-before-commit.png`). On `feature-x` behind `main` the primary is "Update from main" (`/tmp/r3-23-branch-switched.png`).
- Caret menu: PASS. AX `menu More options` with "Pull", "Push", "Pull and push" (shown disabled, no remote) and "Archive workspace" below a divider (`/tmp/r3-17-git-caret-menu.png`). A second press on the caret closed it. No item was pressed.
- Spinner while an action runs: PASS. The primary showed a spinner in place of "Commit" during the commit (`/tmp/r3-18-after-commit.png`), then the toast "Committed".
