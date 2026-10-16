//! GPUI actions for the shortcuts, and their key bindings from `bachuc_command::load_shortcuts`.
//!
//! GPUI cannot remove one binding, and `clear_key_bindings` would also drop the gpui-kit input
//! and menu keys. A reload therefore adds an `Unbind` marker for every old binding, then the new
//! bindings: a marker disables the older bindings of its keys and action, and the newer bindings
//! rank above it.

use crate::state::JUMP_TO_TAB;
use bachuc_command::{Shortcut, gpui_keystrokes};
use gpui_kit::{Action, KeyBinding, Unbind, actions};

/// Runs a shortcut action by its `bachuc_command` action id, for example
/// `workspace.pane.split.right`.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = bachuc, no_json)]
pub struct RunAction(pub String);

/// Shows the tab at this 0-based index of the focused pane.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = bachuc, no_json)]
pub struct JumpToTab(pub usize);

actions!(bachuc, [FindInTimeline]);

/// Key context of the root view. The shortcuts apply anywhere inside it.
pub const SHELL_CONTEXT: &str = "BachucShell";
/// Key context of an agent pane: find, interrupt, and the mode cycle apply only there.
pub const AGENT_CONTEXT: &str = "BachucAgent";

/// Actions bound in the agent pane only. Escape and Shift+Tab mean something else in a
/// terminal or a text field, so they never bind on the whole window.
const AGENT_ACTIONS: [&str; 2] = ["agent.interrupt", "message-input.mode-cycle"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bound {
    Run(String),
    Jump(usize),
}

/// One key binding of the shortcut table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingSpec {
    pub keys: String,
    pub action: Bound,
    pub context: &'static str,
}

impl BindingSpec {
    fn action_name(&self) -> &'static str {
        match self.action {
            Bound::Run(_) => RunAction::name_for_type(),
            Bound::Jump(_) => JumpToTab::name_for_type(),
        }
    }

    pub fn binding(&self) -> KeyBinding {
        match &self.action {
            Bound::Run(id) => {
                KeyBinding::new(&self.keys, RunAction(id.clone()), Some(self.context))
            }
            Bound::Jump(index) => {
                KeyBinding::new(&self.keys, JumpToTab(*index), Some(self.context))
            }
        }
    }

    /// The marker that disables this binding once it is in the keymap.
    pub fn unbinding(&self) -> KeyBinding {
        KeyBinding::new(
            &self.keys,
            Unbind(self.action_name().into()),
            Some(self.context),
        )
    }
}

/// Every key binding of the assigned shortcuts.
pub fn binding_specs(shortcuts: &[Shortcut]) -> Vec<BindingSpec> {
    let mut specs = Vec::new();
    for shortcut in shortcuts {
        let Some(chord) = shortcut.chord.as_deref() else {
            continue;
        };
        let keystrokes = gpui_keystrokes(chord);
        if shortcut.action == JUMP_TO_TAB {
            // `Digit` gives nine strings, for 1 to 9.
            for (index, keys) in keystrokes.into_iter().enumerate() {
                specs.push(BindingSpec {
                    keys,
                    action: Bound::Jump(index),
                    context: SHELL_CONTEXT,
                });
            }
            continue;
        }
        let context = if AGENT_ACTIONS.contains(&shortcut.action.as_str()) {
            AGENT_CONTEXT
        } else {
            SHELL_CONTEXT
        };
        for keys in keystrokes {
            specs.push(BindingSpec {
                keys,
                action: Bound::Run(shortcut.action.clone()),
                context,
            });
        }
    }
    specs
}

/// Bindings that never change: Cmd+F finds in an agent pane, as in Electron.
pub fn fixed_bindings() -> Vec<KeyBinding> {
    vec![KeyBinding::new(
        "cmd-f",
        FindInTimeline,
        Some(AGENT_CONTEXT),
    )]
}

/// The bindings that replace `old` with `new`: markers for the old ones, then the new ones.
pub fn rebind(old: &[BindingSpec], new: &[BindingSpec]) -> Vec<KeyBinding> {
    old.iter()
        .map(BindingSpec::unbinding)
        .chain(new.iter().map(BindingSpec::binding))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_command::default_shortcuts;

    #[test]
    fn every_assigned_shortcut_binds_and_digits_bind_nine_tabs() {
        let shortcuts = default_shortcuts();
        let specs = binding_specs(&shortcuts);
        // Nine digits and one binding for every other action.
        assert_eq!(specs.len(), 9 + shortcuts.len() - 1);
        assert!(specs.contains(&BindingSpec {
            keys: "alt-cmd-1".into(),
            action: Bound::Jump(0),
            context: SHELL_CONTEXT,
        }));
        assert!(specs.contains(&BindingSpec {
            keys: "cmd-k".into(),
            action: Bound::Run("command-center.toggle".into()),
            context: SHELL_CONTEXT,
        }));
    }

    #[test]
    fn escape_and_shift_tab_bind_in_the_agent_pane_only() {
        let specs = binding_specs(&default_shortcuts());
        let interrupt = specs
            .iter()
            .find(|spec| spec.action == Bound::Run("agent.interrupt".into()))
            .expect("bound");
        assert_eq!(interrupt.context, AGENT_CONTEXT);
    }

    #[test]
    fn an_unassigned_shortcut_binds_nothing() {
        let mut shortcuts = default_shortcuts();
        let count = binding_specs(&shortcuts).len();
        shortcuts[0].chord = None;
        assert_eq!(binding_specs(&shortcuts).len(), count - 1);
    }

    #[test]
    fn a_rebind_unbinds_every_old_binding_before_the_new_ones() {
        let old = binding_specs(&default_shortcuts());
        let mut shortcuts = default_shortcuts();
        shortcuts[0].chord = Some("Cmd+P".into());
        let new = binding_specs(&shortcuts);
        let bindings = rebind(&old, &new);
        assert_eq!(bindings.len(), old.len() + new.len());
        assert!(
            bindings[..old.len()]
                .iter()
                .all(|binding| gpui_kit::is_unbind(binding.action()))
        );
        assert!(
            bindings[old.len()..]
                .iter()
                .all(|binding| !gpui_kit::is_unbind(binding.action()))
        );
    }
}
