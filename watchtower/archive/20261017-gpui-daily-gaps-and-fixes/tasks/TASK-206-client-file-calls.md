# TASK-206 Client file calls

Group: C (shared files [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs), [lib.rs](packages/desktop-gpui/crates/client/src/lib.rs), [model.rs](packages/desktop-gpui/crates/client/src/model.rs))
Class: code

## Brief

Goal: The client can suggest files and folders, create, rename, duplicate, and delete entries, and discard changes.

Change: no file write or suggestion calls -> the calls the file menu, @file mentions, the Files search, and terminal links need.

Boundaries:

- `directory_suggestions_request` ([packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) line 2578, response line 6068): fields `query`, `cwd`, `includeFiles`, `includeDirectories`, `matchMode`, `limit` (1 to 100). Keep `matchMode` unset unless the protocol needs it.
- `fs.entry.create.request` (line 2897), `fs.entry.rename.request` (2906), `fs.entry.duplicate.request` (2914), `fs.entry.delete.request` (2921). Gate create, rename, and delete on `server_info.features.fsEntryOps`, and duplicate on `features.fsEntryDuplicate` (lines 3832-3834).
- `checkout.discard_changes.request` (line 2347) with `cwd` and at least one path.
- Download is out of scope.

How:

- Add each request to `core.rs` with a `Pending::*` reply and a `lib.rs` wrapper.
- Add support checks that read `server_info`.
- Add one test per call in `tests/rpcs.rs`.

Files:

- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (requests and replies)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (wrappers)
- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) (new types)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (tests)

Interfaces:

- Produces: `DirectorySuggestion { path: String, is_directory: bool }` and `Client::directory_suggestions(&mut self, cwd: &str, query: &str, include_files: bool, include_directories: bool, limit: u32, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<DirectorySuggestion>>>` for TASK-214, TASK-215, and TASK-203.
- Produces: `NewEntryKind` enum `{ File, Directory }` and `Client::create_entry(&mut self, cwd: &str, parent_path: &str, name: &str, kind: NewEntryKind, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` for TASK-213.
- Produces: `Client::rename_entry(&mut self, cwd: &str, path: &str, name: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`, `Client::duplicate_entry(&mut self, cwd: &str, path: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`, and `Client::delete_entry(&mut self, cwd: &str, path: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` for TASK-213.
- Produces: `Client::fs_entry_ops_supported(&self) -> bool` and `Client::fs_entry_duplicate_supported(&self) -> bool` for TASK-213.
- Produces: `Client::discard_changes(&mut self, cwd: &str, paths: Vec<String>, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` for TASK-213.

Expected result:

- Each call sends the exact request type and fields, proven by `tests/rpcs.rs`.
- `discard_changes` with an empty path list gives `Err` without a request.
- A reply with `error` set gives `Err` with the daemon message.

Anti-goal: the client crate test time must not grow by more than 20%; drift gauge; read `./cargo.sh test -p bachuc-client` wall time before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> passes, with a test per new call and the empty-paths case.
- Anti-goal: test wall time -> at most 120% of the earlier reading.
