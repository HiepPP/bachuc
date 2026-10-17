//! The file open in the Turn diff panel, shared by the cards and the panel (`selection.ts`).
//! The panel API opens a panel by id only, so a pressed file travels through this store.

use crate::turn::{FileRequest, TurnSource};
use gpui_kit::{App, AppContext as _, Context, Entity, Subscription};
use std::cell::RefCell;
use std::rc::Rc;

/// A file of one agent's turn: the snapshot trees and the path in them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileSelection {
    pub agent_id: String,
    pub file: FileRequest,
}

/// Whether a card row is the open file: the agent, `to` tree, and path all match
/// (`isSelected`). A turn with no source matches nothing.
pub(crate) fn selected_row(
    selected: Option<&FileSelection>,
    agent_id: &str,
    source: Option<&TurnSource>,
    path: &str,
) -> bool {
    selected.is_some_and(|selected| {
        selected.agent_id == agent_id
            && source.is_some_and(|source| source.to == selected.file.to)
            && selected.file.path == path
    })
}

/// Only its `notify` matters: views observe it to draw again when the selection changes.
struct Revision;

#[derive(Clone)]
pub(crate) struct Selection {
    current: Rc<RefCell<Option<FileSelection>>>,
    revision: Entity<Revision>,
}

impl Selection {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            current: Rc::default(),
            revision: cx.new(|_| Revision),
        }
    }

    pub(crate) fn get(&self) -> Option<FileSelection> {
        self.current.borrow().clone()
    }

    /// The open file when it belongs to `agent_id`.
    pub(crate) fn for_agent(&self, agent_id: &str) -> Option<FileSelection> {
        self.get()
            .filter(|selection| selection.agent_id == agent_id)
    }

    /// Stores the selection and redraws the observers when it changed.
    pub(crate) fn set(&self, next: Option<FileSelection>, cx: &mut App) {
        if *self.current.borrow() == next {
            return;
        }
        *self.current.borrow_mut() = next;
        self.revision.update(cx, |_, cx| cx.notify());
    }

    /// Runs `on_change` on the view that calls this whenever the selection changes.
    pub(crate) fn observe<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_change: impl Fn(&mut V, &mut Context<V>) + 'static,
    ) -> Subscription {
        cx.observe(&self.revision, move |this, _, cx| on_change(this, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::{FileSelection, selected_row};
    use crate::turn::{FileRequest, TurnSource};

    fn selection(agent_id: &str, to: &str, path: &str) -> FileSelection {
        FileSelection {
            agent_id: agent_id.into(),
            file: FileRequest {
                root: "/repo".into(),
                from: "f".repeat(40),
                to: to.into(),
                path: path.into(),
            },
        }
    }

    fn source(to: &str) -> TurnSource {
        TurnSource {
            workspace_id: "w1".into(),
            root: "/repo".into(),
            from: "f".repeat(40),
            to: to.into(),
        }
    }

    #[test]
    fn selected_row_rule() {
        let open = selection("a1", "t1", "src/a.rs");
        let this_turn = source("t1");
        assert!(selected_row(
            Some(&open),
            "a1",
            Some(&this_turn),
            "src/a.rs"
        ));
        // Another path, agent, or `to` tree is not the open file.
        assert!(!selected_row(
            Some(&open),
            "a1",
            Some(&this_turn),
            "src/b.rs"
        ));
        assert!(!selected_row(
            Some(&open),
            "a2",
            Some(&this_turn),
            "src/a.rs"
        ));
        assert!(!selected_row(
            Some(&open),
            "a1",
            Some(&source("t2")),
            "src/a.rs"
        ));
        // No selection, or a turn with no source, matches nothing.
        assert!(!selected_row(None, "a1", Some(&this_turn), "src/a.rs"));
        assert!(!selected_row(Some(&open), "a1", None, "src/a.rs"));
    }
}
