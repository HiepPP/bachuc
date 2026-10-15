//! GPUI actions for the shortcuts, and their key bindings.

use crate::state::{Shortcut, shortcuts};
use gpui_kit::{Action, KeyBinding, actions};

actions!(
    bachuc,
    [
        NewAgentAction,
        NewTerminalAction,
        CloseTabAction,
        CloseDialogAction
    ]
);

/// Shows the tab at this 0-based index.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = bachuc, no_json)]
pub struct SelectTabAction(pub usize);

/// Key context of the root view. The shortcuts apply anywhere inside it.
pub const SHELL_CONTEXT: &str = "BachucShell";
/// Key context of the new-agent dialog.
pub const DIALOG_CONTEXT: &str = "BachucNewAgent";

pub fn key_bindings() -> Vec<KeyBinding> {
    let mut bindings: Vec<KeyBinding> = shortcuts()
        .into_iter()
        .map(|(keys, shortcut)| match shortcut {
            Shortcut::NewAgent => KeyBinding::new(&keys, NewAgentAction, Some(SHELL_CONTEXT)),
            Shortcut::NewTerminal => KeyBinding::new(&keys, NewTerminalAction, Some(SHELL_CONTEXT)),
            Shortcut::CloseTab => KeyBinding::new(&keys, CloseTabAction, Some(SHELL_CONTEXT)),
            Shortcut::SelectTab(index) => {
                KeyBinding::new(&keys, SelectTabAction(index), Some(SHELL_CONTEXT))
            }
        })
        .collect();
    bindings.push(KeyBinding::new(
        "escape",
        CloseDialogAction,
        Some(DIALOG_CONTEXT),
    ));
    bindings
}
