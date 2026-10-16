//! The tab context menu, a port of `buildWorkspaceTabMenuEntries` for the desktop surface in
//! `packages/app/src/screens/workspace/workspace-tab-menu.ts`, and the per-tab data the menu and
//! the tab chip need beyond `PaneItem`. It holds no GPUI state, so the rules are plain functions.

use bachuc_icons::IconName;
use gpui_kit::Image;
use std::collections::HashMap;
use std::sync::Arc;

/// What a tab shows, which decides its menu. The app sets it with `Workspace::set_item_kind`; a
/// tab without one gets the close items only.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TabKind {
    Agent {
        id: String,
    },
    Terminal {
        id: String,
    },
    File,
    #[default]
    Other,
}

/// One menu item, by what it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabMenuAction {
    CopyResumeCommand,
    CopyAgentId,
    CopyTerminalId,
    CopyFilePath,
    Rename,
    CloseBefore,
    CloseAfter,
    CloseOthers,
    ReloadAgent,
    Close,
}

/// A menu item the app owns, emitted by the `Workspace` with the tab it came from. The close
/// items are not here: the workspace closes the tabs itself and emits `PaneEvent::Closed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabMenuEvent {
    CopyResumeCommand { key: String },
    CopyAgentId { key: String },
    CopyTerminalId { key: String },
    CopyFilePath { key: String },
    Rename { key: String },
    ReloadAgent { key: String },
}

impl TabMenuAction {
    /// The event the app handles for this item, or `None` when the workspace handles it.
    pub fn event(self, key: &str) -> Option<TabMenuEvent> {
        let key = key.to_owned();
        match self {
            TabMenuAction::CopyResumeCommand => Some(TabMenuEvent::CopyResumeCommand { key }),
            TabMenuAction::CopyAgentId => Some(TabMenuEvent::CopyAgentId { key }),
            TabMenuAction::CopyTerminalId => Some(TabMenuEvent::CopyTerminalId { key }),
            TabMenuAction::CopyFilePath => Some(TabMenuEvent::CopyFilePath { key }),
            TabMenuAction::Rename => Some(TabMenuEvent::Rename { key }),
            TabMenuAction::ReloadAgent => Some(TabMenuEvent::ReloadAgent { key }),
            TabMenuAction::CloseBefore
            | TabMenuAction::CloseAfter
            | TabMenuAction::CloseOthers
            | TabMenuAction::Close => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabMenuItem {
    /// The `key` of the entry in `workspace-tab-menu.ts`.
    pub key: &'static str,
    pub action: TabMenuAction,
    pub label: &'static str,
    pub icon: IconName,
    /// Trailing muted text, the first 7 characters of an agent or terminal id.
    pub hint: Option<String>,
    pub tooltip: Option<&'static str>,
    pub disabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabMenuEntry {
    Item(TabMenuItem),
    Separator { key: &'static str },
}

fn short_id(id: &str) -> String {
    id.chars().take(7).collect()
}

/// The menu of the tab at `index` among `count` tabs of its group, in the order of the Electron
/// menu. The close-left, close-right, and close-others items are disabled at the edges.
pub fn menu_entries(kind: &TabKind, index: usize, count: usize) -> Vec<TabMenuEntry> {
    let item = |key, action, label, icon| TabMenuItem {
        key,
        action,
        label,
        icon,
        hint: None,
        tooltip: None,
        disabled: false,
    };
    let mut entries = Vec::new();
    match kind {
        TabKind::Agent { id } => {
            entries.push(TabMenuEntry::Item(item(
                "copy-resume-command",
                TabMenuAction::CopyResumeCommand,
                "Copy resume command",
                IconName::Copy,
            )));
            entries.push(TabMenuEntry::Item(TabMenuItem {
                hint: Some(short_id(id)),
                ..item(
                    "copy-agent-id",
                    TabMenuAction::CopyAgentId,
                    "Copy agent id",
                    IconName::Copy,
                )
            }));
        }
        TabKind::Terminal { id } => {
            entries.push(TabMenuEntry::Item(TabMenuItem {
                hint: Some(short_id(id)),
                ..item(
                    "copy-terminal-id",
                    TabMenuAction::CopyTerminalId,
                    "Copy terminal id",
                    IconName::Copy,
                )
            }));
        }
        TabKind::File => entries.push(TabMenuEntry::Item(item(
            "copy-file-path",
            TabMenuAction::CopyFilePath,
            "Copy file path",
            IconName::Copy,
        ))),
        TabKind::Other => {}
    }
    if matches!(kind, TabKind::Agent { .. } | TabKind::Terminal { .. }) {
        entries.push(TabMenuEntry::Item(item(
            "rename",
            TabMenuAction::Rename,
            "Rename",
            IconName::Pencil,
        )));
        entries.push(TabMenuEntry::Separator {
            key: "rename-separator",
        });
    }
    entries.push(TabMenuEntry::Item(TabMenuItem {
        disabled: index == 0,
        ..item(
            "close-before",
            TabMenuAction::CloseBefore,
            "Close to the left",
            IconName::ArrowLeftToLine,
        )
    }));
    entries.push(TabMenuEntry::Item(TabMenuItem {
        disabled: index + 1 >= count,
        ..item(
            "close-after",
            TabMenuAction::CloseAfter,
            "Close to the right",
            IconName::ArrowRightToLine,
        )
    }));
    entries.push(TabMenuEntry::Item(TabMenuItem {
        disabled: count <= 1,
        ..item(
            "close-others",
            TabMenuAction::CloseOthers,
            "Close other tabs",
            IconName::CopyX,
        )
    }));
    if matches!(kind, TabKind::Agent { .. }) {
        entries.push(TabMenuEntry::Item(TabMenuItem {
            tooltip: Some("Reload agent to update skills, MCPs or login status."),
            ..item(
                "reload-agent",
                TabMenuAction::ReloadAgent,
                "Reload agent",
                IconName::RotateCw,
            )
        }));
    }
    entries.push(TabMenuEntry::Item(item(
        "close",
        TabMenuAction::Close,
        "Close",
        IconName::X,
    )));
    entries
}

/// The keys a close item closes, given the keys of the tab's group in order and the tab's index.
pub fn close_targets(action: TabMenuAction, keys: &[String], index: usize) -> Vec<String> {
    match action {
        TabMenuAction::Close => keys.get(index).cloned().into_iter().collect(),
        TabMenuAction::CloseBefore => keys[..index.min(keys.len())].to_vec(),
        TabMenuAction::CloseAfter => keys[(index + 1).min(keys.len())..].to_vec(),
        TabMenuAction::CloseOthers => keys
            .iter()
            .enumerate()
            .filter(|(at, _)| *at != index)
            .map(|(_, key)| key.clone())
            .collect(),
        _ => Vec::new(),
    }
}

/// The logo and kind of each open tab, by key. `PaneItem` stays as the app builds it, so the
/// extras live beside the layout and `Workspace` drops them when a tab closes.
#[derive(Default)]
pub struct TabExtras {
    logos: HashMap<String, Arc<Image>>,
    kinds: HashMap<String, TabKind>,
}

impl TabExtras {
    /// Sets or clears the logo of an open tab. Returns false, and changes nothing, when `open`
    /// is false because the key is not open.
    pub fn set_logo(&mut self, open: bool, key: &str, logo: Option<Arc<Image>>) -> bool {
        if !open {
            return false;
        }
        match logo {
            Some(logo) => self.logos.insert(key.to_owned(), logo),
            None => self.logos.remove(key),
        };
        true
    }

    /// Sets the kind of an open tab. Returns false when `open` is false.
    pub fn set_kind(&mut self, open: bool, key: &str, kind: TabKind) -> bool {
        if !open {
            return false;
        }
        self.kinds.insert(key.to_owned(), kind);
        true
    }

    pub fn logo(&self, key: &str) -> Option<&Arc<Image>> {
        self.logos.get(key)
    }

    pub fn kind(&self, key: &str) -> TabKind {
        self.kinds.get(key).cloned().unwrap_or_default()
    }

    pub fn forget(&mut self, key: &str) {
        self.logos.remove(key);
        self.kinds.remove(key);
    }

    pub fn is_empty(&self) -> bool {
        self.logos.is_empty() && self.kinds.is_empty()
    }
}
