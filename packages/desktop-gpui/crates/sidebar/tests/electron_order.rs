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

/// Reads the owner's real store (a copy of it) and prints the key counts and the time. Run with
/// `--ignored --nocapture`.
#[test]
#[ignore = "reads the real Electron store of this machine"]
fn the_real_store_reads_in_under_300_ms() {
    let user_data = bachuc_sidebar::electron_order::electron_user_data_dir().expect("home");
    let start = std::time::Instant::now();
    let order = read_electron_order(&user_data).expect("order in the real store");
    let elapsed = start.elapsed();
    println!(
        "projectOrder {} keys, workspaceOrderByProject {} projects ({} keys), \
         pinnedWorkspaceOrder {} keys, read in {elapsed:?}",
        order.project_order.len(),
        order.workspace_order_by_project.len(),
        order
            .workspace_order_by_project
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        order.pinned_workspace_order.len(),
    );
    assert!(!order.project_order.is_empty());
    assert!(
        elapsed < std::time::Duration::from_millis(300),
        "{elapsed:?}"
    );
}
