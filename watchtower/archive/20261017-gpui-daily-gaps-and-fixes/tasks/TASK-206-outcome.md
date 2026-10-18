# TASK-206 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/model.rs`: new types `DirectorySuggestion` and `NewEntryKind`.
- `packages/desktop-gpui/crates/client/src/core.rs`: `Pending::DirectorySuggestions` and `Pending::FsEntry`, the suggestion, entry, and discard calls, the two feature checks, and a `failed` helper for calls that send no request. `fork_context` uses the helper too; its behavior is the same.
- `packages/desktop-gpui/crates/client/src/lib.rs`: the exports and one `Client` wrapper per call.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: 7 new tests.

## Contract

- Every "Produces:" line is implemented with its exact name and signature. `ClientCore` has the same methods without `cx`.
- `directory_suggestions` sends `cwd`, `query`, `includeFiles`, `includeDirectories`, and `limit` clamped to 1..=100. It sends no `matchMode`. It reads `entries`; an entry of an unknown kind is skipped. If `entries` is empty, it reads `directories` as folders, the same rule as the app's `mapDirectorySuggestionsToEntries`.
- `create_entry`, `rename_entry`, and `delete_entry` need `features.fsEntryOps`. `duplicate_entry` needs `features.fsEntryDuplicate`. Without the feature, the call fails with "Update the host to use this." and sends no request.
- An `fs.entry.*` answer with a string `error` gives `Err(error)`. An answer with `success: false` and no error gives `Err("the file operation failed")`. The calls return `()`, so the new path (`renamedPath`, `duplicatedPath`, `path`) is dropped.
- `discard_changes` with no paths fails with "No files to discard" and sends no request. A checkout `{code, message}` error gives `Err(message)`.
- `discard_changes` is not gated on `features.checkoutDiscardChanges`, because the spec names no gate and no support check for it.

## Verified

- Baseline `./cargo.sh test -p bachuc-client`, warm: 15.40 s wall, passed. Test execution summed to 9.22 s. Load average was near 10.
- After the change, `./cargo.sh test -p bachuc-client` passed, with 52 tests in rpcs (45 before plus 7 new). The warm run took 6.81 s wall (44% of baseline). The first run after the change took 18.61 s because it recompiled the crate. The anti-goal holds.
- New tests: one each for suggestions, create, rename, duplicate (with its gate), delete, and discard (with the empty-paths case), plus one for the `fsEntryOps` gate.
- `./cargo.sh fmt --check -p bachuc-client` exited 0.
- `clippy -p bachuc-client --tests` gave no client warnings. The 2 warnings come from the existing `bachuc-protocol` build script.
- `./cargo.sh metadata --format-version 1` exited 0. `./cargo.sh build -p bachuc-app` exited 0.
- No UI checks apply to this TASK.

## Lessons

- The fake daemon fills in `requestId`, so a test reply gives only the payload fields.
- If TASK-213 needs the new path after a rename or duplicate, `rename_entry` and `duplicate_entry` need a return value; the reply has `renamedPath` and `duplicatedPath`.
- If TASK-213 must hide Discard on an old host, it needs a new `discard_changes_supported` check on `features.checkoutDiscardChanges`.
