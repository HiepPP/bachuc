# TASK-279 Outcome

## Status

DONE (code). The UI checker line is DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/sidebar/Cargo.toml`: `rusty-leveldb = "4"` (resolves to 4.0.2, default `fs` feature, includes snappy).
- `packages/desktop-gpui/Cargo.lock`: 7 new packages (`rusty-leveldb`, `snap`, `crc32c`, `errno 0.2.8`, `errno-dragonfly`, `fs2`, `integer-encoding`) and the `rusty-leveldb` entry in `bachuc-sidebar`. Two older `errno` references got a version suffix (`errno 0.3.14`), because a second `errno` now exists. The tree was clean at the start, so no other hunk is in the diff.
- `packages/desktop-gpui/crates/sidebar/src/electron_order.rs` (new): `parse_order_value`, `read_electron_order`, `electron_user_data_dir`.
- `packages/desktop-gpui/crates/sidebar/src/lib.rs`: `pub mod electron_order`.
- `packages/desktop-gpui/crates/sidebar/src/prefs.rs`: `electron_order_imported` (JSON `electronOrderImported`), `SidebarPrefs::load_with_import`, and the three prefs tests plus a flag round-trip test.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: `with_data_dir` calls `load_with_import(dir, electron_user_data_dir().as_deref())` in place of `load_from`.
- `packages/desktop-gpui/crates/sidebar/tests/electron_order.rs` (new): the five red tests, plus the `#[ignore]` real-store test.

## Contract

- Produces `electron_order::parse_order_value(&[u8]) -> Option<SidebarOrder>`, `read_electron_order(&Path) -> Option<SidebarOrder>`, `electron_user_data_dir() -> Option<PathBuf>`, `SidebarPrefs.electron_order_imported: bool`, `SidebarPrefs::load_with_import(&Path, Option<&Path>) -> SidebarPrefs`.
- Consumes `SidebarOrder`, `ViewState.order`, and the three `sidebar.json` keys exactly as TASK-278 shipped them (`seed_order` is not used here).
- `parse_order_value` returns `None` for an empty value, an unknown prefix byte, text that is not JSON, and JSON without a `state` object. A missing or bad field inside `state` gives an empty value. Safe default: a value with no `state` object counts as bad, so it never wipes the saved order.
- `read_electron_order` copies the files of `<user data>/Local Storage/leveldb` (skipping `LOCK` and non-files) into `$TMPDIR/bachuc-electron-order-<pid>-<nanos>`, opens the copy with `create_if_missing: false`, reads the key, and removes the copy through a drop guard. A copy or open failure and a bad value log a warning. A missing folder or key gives `None` without a warning.
- `load_with_import` with `Some(user_data)` and the flag unset: reads once, replaces `view.order` on success, sets the flag, and saves with `save_to` (also when the read gave nothing). With the flag set it reads nothing and writes nothing. With `None` (no `HOME`) it skips the import and leaves the flag unset, so a later launch with a home folder can still import. This is a choice the spec did not name.
- The import runs on the main thread inside `with_data_dir`, before `attach_host` and the first `refresh`.

## Verified

- Red: with no `electron_order` module the test file failed to compile (E0432 unresolved import). Counts as red.
- `./cargo.sh test -p bachuc-sidebar --test electron_order` -> 5 passed, 1 ignored.
- `./cargo.sh test -p bachuc-sidebar --lib` -> 19 passed. The three spec tests are `a_missing_electron_folder_sets_the_flag_keeps_the_order_and_writes_the_file`, `the_import_replaces_the_order_once`, and `a_set_flag_never_reads_the_store`; the flag test is `the_import_flag_round_trips_and_defaults_to_false`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0. `git diff --stat packages/desktop-gpui/Cargo.lock` -> 71 insertions, 3 deletions (the `errno` suffix lines above). The "line already there" from the spec is gone, because the other thread committed it in `c17427731`.
- `./cargo.sh clippy -p bachuc-sidebar --no-deps --lib --test model --test electron_order -- -D warnings -A clippy::too_many_arguments` -> clean. The unmodified spec command also fails on code older than this TASK (see TASK-278 outcome).
- `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- UI checker (check bundle on the release host, AX row order): DEFERRED (UI checker).
- Snappy and the bytewise comparator: confirmed on the real store. `rusty-leveldb` 4.0.2 opened Chromium's folder (`.ldb` tables, `.log`, `MANIFEST-000001`) and read the value that crosses a log block.

## Anti-goal

Real store, read only. Test: `the_real_store_reads_in_under_300_ms` (`#[ignore]`), run with `./cargo.sh test -p bachuc-sidebar --test electron_order -- --ignored --nocapture`.

| Checkpoint                                                                      | Value                                                                                                                                                                       | Source               | Time                |
| ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------- | ------------------- |
| `ls -lT` of `~/Library/Application Support/Bachuc/Local Storage/leveldb` before | 9 entries: `000005.ldb` 16571, `005161.ldb` 995577, `005162.log` 1104633, `005163.ldb` 172646, `CURRENT` 16, `LOCK` 0, `LOG` 313, `LOG.old` 20462, `MANIFEST-000001` 315221 | `/tmp/ls-before.txt` | 2026-10-20 14:55:33 |
| `read_electron_order` on the real store                                         | `projectOrder` 52 keys; `workspaceOrderByProject` 49 projects, 1097 keys; `pinnedWorkspaceOrder` 0 keys; 84.6 ms (debug build)                                              | ignored test output  | 2026-10-20 14:55:35 |
| `ls -lT` after                                                                  | identical to before (`diff` empty): same names, sizes, and times                                                                                                            | `/tmp/ls-after.txt`  | 2026-10-20 14:55:36 |

Time 84.6 ms is under 300 ms. The folder is unchanged. The test reads a temp copy, which was removed (no `bachuc-electron-order-*` left in `/tmp`). Electron was not running its writes during the read (its `LOG` time stayed 14:20:31). The 300 ms start cost is one real-store read in a debug build; the first GPUI start with the flag cleared (UI checker) is DEFERRED (UI checker). PASS for the read-only part.

## Lessons

- `rusty-leveldb` 4.0.2 reads Chromium's Local Storage as it is, snappy blocks and all, when the folder is a copy without `LOCK`.
- The value holds 49 project entries and 1097 thread keys; the project order has 52 keys, so a few projects in the order are not in the GPUI list. TASK-278 keeps them and does not prune.
- `Cargo.lock` gets an `errno 0.3.14` rename on the two existing references when `errno 0.2.8` enters; a Verify line that expects "only new entries" has to allow that.
- Prefs tests that need a LevelDB folder build one with `rusty_leveldb::DB::open` in a temp folder (the `electron_store` helper in `prefs.rs`).

## Group-level run

One run for TASK-278, TASK-279, and TASK-280 together (2026-10-20 14:59 to 15:00, after the last TASK, `CARGO_BUILD_JOBS=4`):

- `./cargo.sh test -p bachuc-sidebar` -> lib 19, `electron_order` 5 (1 ignored), `fake_daemon` 1, `model` 55, `plugin` 32 passed. One run of the `model` suite failed `building_500_workspaces_in_status_mode_stays_under_2_ms` under load (the suite runs on several threads). Alone it passes at 1.32 and 1.35 ms, and a rerun of the whole suite passes. Status grouping code is unchanged by the group (`sorted` returns at once with an empty order).
- `./cargo.sh check -p bachuc-app -p bachuc-composer` -> finished, no error. These crates use `bachuc-sidebar`.
- `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- No `cargo clean` was run; the lead does it.

## UI Checks

2026-10-20 15:04 to 15:08, lead by `cua_repl`:

- `sidebar.json` backed up to `/tmp/wt-sidebar-backup.json`. It had no `electronOrderImported` key, so the flag read as false.
- The check bundle started with `BACHUC_HOST=release`. Within 8 s `sidebar.json` held `electronOrderImported: true` and 52 `projectOrder` keys, the same count as the real-store read. The import ran at start.
- AX row order: UNVERIFIED (unattended). The background window kept a stale "Connecting" frame on release and then on live, and the AX tree held only the window frame. The main thread was idle (`sample`), and the live log showed `Client connected via hello` for this client at 15:06:45. A raise, a title-bar click, and `Escape` did not repaint it. See Q-182.
- The check app on live pruned `collapsedProjects` in the shared `sidebar.json`. The lead restored the backup byte for byte and quit the check app.

## Lead Check

- Same re-check as TASK-278. `Cargo.lock` diff holds only the 7 new packages, the `rusty-leveldb` line, and the `errno 0.3.14` suffix on 2 old references. `./cargo.sh metadata` exit 0 was the builder's read.
- Committed in `9cc6e3d50`. Status: DONE, implement: manual checks pending (Q-182).
