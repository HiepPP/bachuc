use bachuc_icons::IconName;
use bachuc_workspace::TabMenuEvent;
use bachuc_workspace::tab_menu::{
    TabExtras, TabKind, TabMenuAction, TabMenuEntry, TabMenuItem, close_targets, menu_entries,
};
use gpui_kit::{Image, ImageFormat};
use std::path::PathBuf;
use std::sync::Arc;

fn repo_file(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../app/src");
    std::fs::read_to_string(root.join(path)).expect("the Electron source file exists")
}

fn items(entries: &[TabMenuEntry]) -> Vec<&TabMenuItem> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            TabMenuEntry::Item(item) => Some(item),
            TabMenuEntry::Separator { .. } => None,
        })
        .collect()
}

fn keys(entries: &[TabMenuEntry]) -> Vec<&'static str> {
    entries
        .iter()
        .map(|entry| match entry {
            TabMenuEntry::Item(item) => item.key,
            TabMenuEntry::Separator { key } => key,
        })
        .collect()
}

fn agent() -> TabKind {
    TabKind::Agent {
        id: "0123456789abcdef".into(),
    }
}

fn terminal() -> TabKind {
    TabKind::Terminal {
        id: "fedcba9876543210".into(),
    }
}

#[test]
fn each_kind_gets_the_electron_items_in_the_electron_order() {
    assert_eq!(
        keys(&menu_entries(&agent(), 1, 3)),
        [
            "copy-resume-command",
            "copy-agent-id",
            "rename",
            "rename-separator",
            "close-before",
            "close-after",
            "close-others",
            "reload-agent",
            "close",
        ]
    );
    assert_eq!(
        keys(&menu_entries(&terminal(), 1, 3)),
        [
            "copy-terminal-id",
            "rename",
            "rename-separator",
            "close-before",
            "close-after",
            "close-others",
            "close",
        ]
    );
    assert_eq!(
        keys(&menu_entries(&TabKind::File, 1, 3)),
        [
            "copy-file-path",
            "close-before",
            "close-after",
            "close-others",
            "close"
        ]
    );
    assert_eq!(
        keys(&menu_entries(&TabKind::Other, 1, 3)),
        ["close-before", "close-after", "close-others", "close"]
    );
}

#[test]
fn the_items_match_workspace_tab_menu_ts() {
    let source = repo_file("screens/workspace/workspace-tab-menu.ts");
    let en = repo_file("i18n/resources/en.ts");
    // The entry keys in source order, each once.
    let mut ts_keys: Vec<&str> = Vec::new();
    for part in source.split("key: \"").skip(1) {
        let key = part.split('"').next().unwrap();
        if !ts_keys.contains(&key) {
            ts_keys.push(key);
        }
    }
    // (key, TS icon, `WorkspaceTabMenuLabels` field, English label)
    let table = [
        (
            "copy-resume-command",
            "copy",
            "copyResumeCommand",
            "Copy resume command",
        ),
        ("copy-agent-id", "copy", "copyAgentId", "Copy agent id"),
        (
            "copy-terminal-id",
            "copy",
            "copyTerminalId",
            "Copy terminal id",
        ),
        ("copy-file-path", "copy", "copyFilePath", "Copy file path"),
        ("rename", "pencil", "rename", "Rename"),
        (
            "close-before",
            "arrow-left-to-line",
            "closeLeft",
            "Close to the left",
        ),
        (
            "close-after",
            "arrow-right-to-line",
            "closeRight",
            "Close to the right",
        ),
        ("close-others", "copy-x", "closeOthers", "Close other tabs"),
        ("reload-agent", "rotate-cw", "reloadAgent", "Reload agent"),
        ("close", "x", "close", "Close"),
    ];
    let icon_of = |ts: &str| match ts {
        "copy" => IconName::Copy,
        "pencil" => IconName::Pencil,
        "arrow-left-to-line" => IconName::ArrowLeftToLine,
        "arrow-right-to-line" => IconName::ArrowRightToLine,
        "copy-x" => IconName::CopyX,
        "rotate-cw" => IconName::RotateCw,
        "x" => IconName::X,
        other => panic!("unmapped icon {other}"),
    };

    for kind in [agent(), terminal(), TabKind::File, TabKind::Other] {
        let entries = menu_entries(&kind, 1, 3);
        // The kind's keys keep the order of the TS source.
        let mut at = 0;
        for key in keys(&entries) {
            let found = ts_keys[at..]
                .iter()
                .position(|candidate| *candidate == key)
                .unwrap_or_else(|| panic!("{key} is missing or out of order in the TS menu"));
            at += found + 1;
        }
        for item in items(&entries) {
            let (_, ts_icon, field, label) = table
                .iter()
                .find(|row| row.0 == item.key)
                .unwrap_or_else(|| panic!("{} has no table row", item.key));
            assert_eq!(item.label, *label, "{} label", item.key);
            assert_eq!(item.icon, icon_of(ts_icon), "{} icon", item.key);
            assert!(
                en.contains(&format!("{field}: \"{label}\"")),
                "{field} is \"{label}\" in en.ts"
            );
            assert!(
                source.contains(&format!("labels.{field}")),
                "{field} is used"
            );
            let block = source
                .split(&format!("key: \"{}\"", item.key))
                .nth(1)
                .unwrap();
            let block = &block[..block.find("onSelect").unwrap()];
            assert!(
                block.contains(&format!("icon: \"{ts_icon}\"")),
                "{} icon in TS",
                item.key
            );
            assert_eq!(
                block.contains("tooltip:"),
                item.tooltip.is_some(),
                "{} tooltip",
                item.key
            );
            assert_eq!(
                block.contains("hint:"),
                item.hint.is_some(),
                "{} hint",
                item.key
            );
        }
    }
    assert!(
        en.contains("reloadAgentTooltip: \"Reload agent to update skills, MCPs or login status.\"")
    );
    let reload = menu_entries(&agent(), 0, 1);
    let reload = items(&reload)
        .into_iter()
        .find(|i| i.key == "reload-agent")
        .unwrap();
    assert_eq!(
        reload.tooltip,
        Some("Reload agent to update skills, MCPs or login status.")
    );
}

#[test]
fn hints_are_the_first_seven_characters_of_the_id() {
    let entries = menu_entries(&agent(), 0, 1);
    let id = items(&entries)
        .into_iter()
        .find(|i| i.key == "copy-agent-id")
        .unwrap();
    assert_eq!(id.hint.as_deref(), Some("0123456"));
    let entries = menu_entries(&terminal(), 0, 1);
    let id = items(&entries)
        .into_iter()
        .find(|i| i.key == "copy-terminal-id")
        .unwrap();
    assert_eq!(id.hint.as_deref(), Some("fedcba9"));
    let short = TabKind::Agent { id: "abc".into() };
    let entries = menu_entries(&short, 0, 1);
    let id = items(&entries)
        .into_iter()
        .find(|i| i.key == "copy-agent-id")
        .unwrap();
    assert_eq!(id.hint.as_deref(), Some("abc"));
}

#[test]
fn close_items_disable_at_the_edges() {
    let disabled = |index, count| {
        let entries = menu_entries(&TabKind::File, index, count);
        let state = |key| {
            items(&entries)
                .into_iter()
                .find(|i| i.key == key)
                .unwrap()
                .disabled
        };
        (
            state("close-before"),
            state("close-after"),
            state("close-others"),
            state("close"),
        )
    };
    assert_eq!(disabled(0, 3), (true, false, false, false));
    assert_eq!(disabled(1, 3), (false, false, false, false));
    assert_eq!(disabled(2, 3), (false, true, false, false));
    assert_eq!(disabled(0, 1), (true, true, true, false));
}

#[test]
fn the_app_owns_every_item_but_the_close_items() {
    for (action, event) in [
        (
            TabMenuAction::CopyResumeCommand,
            TabMenuEvent::CopyResumeCommand { key: "k".into() },
        ),
        (
            TabMenuAction::CopyAgentId,
            TabMenuEvent::CopyAgentId { key: "k".into() },
        ),
        (
            TabMenuAction::CopyTerminalId,
            TabMenuEvent::CopyTerminalId { key: "k".into() },
        ),
        (
            TabMenuAction::CopyFilePath,
            TabMenuEvent::CopyFilePath { key: "k".into() },
        ),
        (
            TabMenuAction::Rename,
            TabMenuEvent::Rename { key: "k".into() },
        ),
        (
            TabMenuAction::ReloadAgent,
            TabMenuEvent::ReloadAgent { key: "k".into() },
        ),
    ] {
        assert_eq!(action.event("k"), Some(event));
    }
    for action in [
        TabMenuAction::CloseBefore,
        TabMenuAction::CloseAfter,
        TabMenuAction::CloseOthers,
        TabMenuAction::Close,
    ] {
        assert_eq!(action.event("k"), None);
    }
}

#[test]
fn close_items_pick_the_tabs_around_the_clicked_one() {
    let keys: Vec<String> = ["a", "b", "c", "d"].map(String::from).into();
    assert_eq!(close_targets(TabMenuAction::Close, &keys, 1), ["b"]);
    assert_eq!(
        close_targets(TabMenuAction::CloseBefore, &keys, 2),
        ["a", "b"]
    );
    assert_eq!(
        close_targets(TabMenuAction::CloseAfter, &keys, 1),
        ["c", "d"]
    );
    assert_eq!(
        close_targets(TabMenuAction::CloseOthers, &keys, 1),
        ["a", "c", "d"]
    );
    assert!(close_targets(TabMenuAction::CloseBefore, &keys, 0).is_empty());
    assert!(close_targets(TabMenuAction::CloseAfter, &keys, 3).is_empty());
    assert!(close_targets(TabMenuAction::Rename, &keys, 1).is_empty());
}

fn logo() -> Arc<Image> {
    Arc::new(Image::from_bytes(ImageFormat::Svg, b"<svg/>".to_vec()))
}

#[test]
fn set_item_logo_on_an_unknown_key_changes_nothing() {
    let mut extras = TabExtras::default();
    assert!(!extras.set_logo(false, "missing", Some(logo())));
    assert!(extras.logo("missing").is_none());
    assert!(extras.is_empty());
    assert!(!extras.set_kind(false, "missing", agent()));
    assert_eq!(extras.kind("missing"), TabKind::Other);
}

#[test]
fn set_item_badge_on_an_unknown_key_changes_nothing() {
    let mut extras = TabExtras::default();
    assert!(!extras.set_badge(false, "missing", None));
    assert!(extras.badge("missing").is_none());
    assert!(extras.is_empty());
}

#[test]
fn set_item_badge_clears_and_forgets() {
    // A badge needs an `Entity`, which needs a running app, so the set and replace cases run
    // only in the app. Clearing and forgetting need no view.
    let mut extras = TabExtras::default();
    assert!(extras.set_badge(true, "a", None));
    assert!(extras.badge("a").is_none());
    extras.set_kind(true, "a", agent());
    extras.forget("a");
    assert!(extras.badge("a").is_none());
    assert!(extras.is_empty());
}

#[test]
fn set_item_logo_sets_replaces_clears_and_forgets() {
    let mut extras = TabExtras::default();
    let first = logo();
    assert!(extras.set_logo(true, "a", Some(first.clone())));
    assert!(Arc::ptr_eq(extras.logo("a").unwrap(), &first));
    let second = Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        b"<svg id=\"2\"/>".to_vec(),
    ));
    assert!(extras.set_logo(true, "a", Some(second.clone())));
    assert!(Arc::ptr_eq(extras.logo("a").unwrap(), &second));
    assert!(extras.set_logo(true, "a", None));
    assert!(extras.logo("a").is_none());

    extras.set_logo(true, "b", Some(logo()));
    extras.set_kind(true, "b", TabKind::File);
    extras.forget("b");
    assert!(extras.is_empty());
}
