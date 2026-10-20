# TASK-279 GPUI imports Electron's saved sidebar order once

Group: A (shared files: `crates/sidebar/src/prefs.rs`, `crates/sidebar/src/sidebar.rs`, with TASK-278 and TASK-280)
Class: code

## Brief

Goal: On its first launch after this change, GPUI copies Electron's saved sidebar order into `sidebar.json`, so the sidebar shows the same order as Electron.

Change: no import -> one import from Electron's LevelDB store, marked done in `sidebar.json`.

Design: [ADR-0057](watchtower/decisions/ADR-0057-gpui-saved-sidebar-order-with-electron-import-over-name-sort.md)

Boundaries:

- The store: `~/Library/Application Support/Bachuc/Local Storage/leveldb`. The key is the bytes `_paseo://app`, `0x00`, `0x01`, then `sidebar-project-workspace-order`. The value is one prefix byte and then the JSON: `0x01` means Latin-1, and `0x00` means UTF-16LE. The JSON is `{"state":{"projectOrder":[...],"workspaceOrderByProject":{...},"pinnedWorkspaceOrder":[...]},"version":N}`. Checked on 2026-10-20 with version 1.
- Read only these three fields. Ignore the older `projectOrderByServerId` and `workspaceOrderByServerAndProject` fields. A missing or bad field gives an empty value.
- Never open Electron's folder in place. `rusty-leveldb` writes a LOCK file, and it can write logs or compact the store. Copy the folder to a new temp folder without the `LOCK` file. Open the copy, then delete the copy.
- The import runs once. Set `electron_order_imported` after the first try, also when the folder is missing or the read fails. A failed read logs a warning and leaves the order as it is.
- Planner call: a successful import replaces `view.order`. Before the import, `sidebar.json` holds only keys that TASK-278 seeded by name, so nothing the owner chose is lost. `seed_order` adds the threads that Electron never saw on the next refresh, and they go first.
- Planner call: the import runs on the main thread when the prefs load. The store is about 1.3 MB.
- Trap: `Sidebar::new` passes no data dir. The real constructor is `with_data_dir` in [sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs), which `crates/app/src/shell.rs` calls. It loads the prefs with `SidebarPrefs::load_from`, and `attach_host` can call `refresh` soon after. So replace that `load_from` call with the new `SidebarPrefs::load_with_import`, before any `refresh` can seed.
- Add `rusty-leveldb` only to [crates/sidebar/Cargo.toml](packages/desktop-gpui/crates/sidebar/Cargo.toml). `Cargo.lock` has an uncommitted line from another thread, so stage only your hunks.

How:

- Add `rusty-leveldb` to the sidebar crate. Check that it opens a store with snappy blocks and the default bytewise comparator.
- Add [crates/sidebar/src/electron_order.rs](packages/desktop-gpui/crates/sidebar/src/electron_order.rs) with `parse_order_value`, `read_electron_order`, and `electron_user_data_dir`. Export the module from [lib.rs](packages/desktop-gpui/crates/sidebar/src/lib.rs).
- In [prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs), add `electron_order_imported: bool` with the JSON key `electronOrderImported`.
- In [prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs), add `SidebarPrefs::load_with_import(dir: &Path, electron_user_data: Option<&Path>) -> SidebarPrefs`. It calls `load_from`. When the flag is false, it reads the store, replaces `view.order` on success, sets the flag, and saves with `save_to`.
- In `with_data_dir` in [sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs), call `load_with_import` with `electron_user_data_dir()` in place of `load_from`. The `sample()` literal in the prefs tests needs the new field.

Files:

- [packages/desktop-gpui/crates/sidebar/Cargo.toml](packages/desktop-gpui/crates/sidebar/Cargo.toml) (the `rusty-leveldb` dependency)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (new crate entries only)
- [packages/desktop-gpui/crates/sidebar/src/electron_order.rs](packages/desktop-gpui/crates/sidebar/src/electron_order.rs) (new: read and parse)
- [packages/desktop-gpui/crates/sidebar/src/lib.rs](packages/desktop-gpui/crates/sidebar/src/lib.rs) (module export)
- [packages/desktop-gpui/crates/sidebar/src/prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs) (the import flag and `load_with_import`)
- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (one call in `with_data_dir`)
- [packages/desktop-gpui/crates/sidebar/tests/electron_order.rs](packages/desktop-gpui/crates/sidebar/tests/electron_order.rs) (new: red tests)

Interfaces:

- Consumes: `bachuc_sidebar::model::SidebarOrder { pub project_order: Vec<String>, pub workspace_order_by_project: HashMap<String, Vec<String>>, pub pinned_workspace_order: Vec<String> }`, which derives `Clone, Debug, Default, PartialEq, Eq`, with `SidebarOrder::is_empty(&self) -> bool`, from TASK-278
- Consumes: `ViewState.order: SidebarOrder` from TASK-278
- Consumes: `bachuc_sidebar::model::seed_order(order: &mut SidebarOrder, projects: &[ProjectNode], facts: &Facts) -> bool` from TASK-278
- Consumes: `sidebar.json` keys `projectOrder: string[]`, `workspaceOrderByProject: Record<string, string[]>`, and `pinnedWorkspaceOrder: string[]` from TASK-278
- Produces: `bachuc_sidebar::electron_order::parse_order_value(value: &[u8]) -> Option<SidebarOrder>`
- Produces: `bachuc_sidebar::electron_order::read_electron_order(user_data: &Path) -> Option<SidebarOrder>`
- Produces: `bachuc_sidebar::electron_order::electron_user_data_dir() -> Option<PathBuf>`
- Produces: `SidebarPrefs.electron_order_imported: bool` (JSON `electronOrderImported`)
- Produces: `SidebarPrefs::load_with_import(dir: &Path, electron_user_data: Option<&Path>) -> SidebarPrefs`

Expected result:

- WHEN GPUI starts and `electronOrderImported` is not true THE sidebar SHALL read Electron's saved order and replace its own order with it.
- WHEN the import ran once THE sidebar SHALL NOT read Electron's store again.
- IF Electron's folder is missing, or the key is missing, or the value is bad THEN THE sidebar SHALL keep its order, set the flag, and log a warning for a bad value.
- WHILE Electron holds its LOCK file, WHEN GPUI imports THE import SHALL still read the copy.
- THE import SHALL leave Electron's folder unchanged.

Red tests:

File: [packages/desktop-gpui/crates/sidebar/tests/electron_order.rs](packages/desktop-gpui/crates/sidebar/tests/electron_order.rs)

```rust
use bachuc_sidebar::electron_order::{parse_order_value, read_electron_order};
use std::path::PathBuf;

const KEY: &[u8] = b"_paseo://app\x00\x01sidebar-project-workspace-order";
const JSON: &str = r#"{"state":{"projectOrder":["remote:github.com/a/b","[\"srv\",\"prj\"]"],"workspaceOrderByProject":{"remote:github.com/a/b":["srv:w2","srv:w1"]},"pinnedWorkspaceOrder":["srv:w1"]},"version":1}"#;

fn latin1(json: &str) -> Vec<u8> {
    let mut value = vec![1u8];
    value.extend(json.bytes());
    value
}

fn temp_user_data() -> PathBuf {
    std::env::temp_dir().join(format!("electron-order-{}", uuid::Uuid::new_v4()))
}

#[test]
fn a_latin1_value_parses_the_three_orders() {
    let order = parse_order_value(&latin1(JSON)).expect("order");
    assert_eq!(
        order.project_order,
        ["remote:github.com/a/b", "[\"srv\",\"prj\"]"]
    );
    assert_eq!(
        order.workspace_order_by_project["remote:github.com/a/b"],
        ["srv:w2", "srv:w1"]
    );
    assert_eq!(order.pinned_workspace_order, ["srv:w1"]);
}

#[test]
fn a_utf16_value_parses_too() {
    let mut value = vec![0u8];
    for unit in JSON.encode_utf16() {
        value.extend(unit.to_le_bytes());
    }
    let order = parse_order_value(&value).expect("order");
    assert_eq!(order.pinned_workspace_order, ["srv:w1"]);
}

#[test]
fn a_bad_value_gives_none_and_a_bad_field_gives_empty() {
    assert!(parse_order_value(b"").is_none());
    assert!(parse_order_value(&latin1("not json")).is_none());
    let order =
        parse_order_value(&latin1(r#"{"state":{"projectOrder":5},"version":1}"#)).expect("order");
    assert!(order.project_order.is_empty());
}

#[test]
fn the_store_reads_from_a_copy_and_stays_unchanged() {
    let user_data = temp_user_data();
    let store = user_data.join("Local Storage/leveldb");
    {
        let options = rusty_leveldb::Options {
            create_if_missing: true,
            ..rusty_leveldb::Options::default()
        };
        let mut db = rusty_leveldb::DB::open(&store, options).expect("open");
        db.put(KEY, &latin1(JSON)).expect("put");
        db.flush().expect("flush");
    }
    let listing = |dir: &PathBuf| {
        let mut entries: Vec<(String, u64)> = std::fs::read_dir(dir)
            .expect("dir")
            .map(|entry| {
                let entry = entry.expect("entry");
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    entry.metadata().expect("meta").len(),
                )
            })
            .collect();
        entries.sort();
        entries
    };
    let before = listing(&store);
    let order = read_electron_order(&user_data).expect("order");
    assert_eq!(order.pinned_workspace_order, ["srv:w1"]);
    assert_eq!(listing(&store), before, "Electron's folder is unchanged");
    std::fs::remove_dir_all(&user_data).expect("cleanup");
}

#[test]
fn a_missing_store_gives_none() {
    assert!(read_electron_order(&temp_user_data()).is_none());
}
```

Move `rusty-leveldb` into dev-dependencies too if the test crate cannot reach it otherwise. Add prefs tests in the `prefs.rs` test module:

- `electronOrderImported` round-trips, and a file without it parses to `false`.
- `load_with_import` with a missing Electron folder sets the flag, keeps the order, and writes `sidebar.json`.
- `load_with_import` with the flag already true does not read the store: pass a store folder that holds a different order, and the saved order stays.

Anti-goal: Electron's store stays unchanged, and the import adds at most 300 ms to GPUI start. Read the file names, sizes, and modified times of `~/Library/Application Support/Bachuc/Local Storage/leveldb` before and after one GPUI start with the flag cleared. Time `read_electron_order` with `std::time::Instant` on the owner's real store. Tripwire: any changed file in that folder.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar --test electron_order` -> all five red tests pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar --lib` -> the three prefs import tests pass.
- `cd packages/desktop-gpui && ./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0, and `git diff packages/desktop-gpui/Cargo.lock` shows only the new crate entries plus the line that was already there.
- Real store, read only: add an `#[ignore]` test that calls `read_electron_order` on `electron_user_data_dir()` and prints the key counts and the time. Run it with `./cargo.sh test -p bachuc-sidebar --test electron_order -- --ignored --nocapture` -> `projectOrder` has more than 0 keys, and the time is under 300 ms.
- Anti-goal: `ls -lT "$HOME/Library/Application Support/Bachuc/Local Storage/leveldb"` before and after the ignored test -> the same names, sizes, and times.
- UI checker: back up `~/Library/Application Support/BachucGpui/sidebar.json`, then set `electronOrderImported` to `false` in it. Start a check bundle with `BACHUC_HOST` set to the release host. Read the sidebar row order through AX -> the projects follow Electron's `projectOrder`. In the first project, the threads that Electron's saved order lists keep that order, after any threads it does not list. Restore `sidebar.json` and quit the check app.
- After the checks, run `./cargo.sh clean --profile dev` when no other `cargo` or `rustc` process runs.
