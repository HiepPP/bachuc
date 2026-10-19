//! GPUI actions for the shortcuts, and their key bindings from `bachuc_command::load_shortcuts`.
//!
//! GPUI cannot remove one binding, and `clear_key_bindings` would also drop the gpui-kit input
//! and menu keys. A reload therefore adds an `Unbind` marker for every old binding, then the new
//! bindings: a marker disables the older bindings of its keys and action, and the newer bindings
//! rank above it.

use crate::state::JUMP_TO_TAB;
use bachuc_command::{Shortcut, gpui_keystrokes};
use bachuc_workspace::KEY_CONTEXT as WORKSPACE_CONTEXT;
use bachuc_workspace::actions::OpenTabMenu;
use gpui_kit::base::input;
use gpui_kit::{Action, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType, Unbind, actions};

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

// The app and window menu items. The shell shortcut table has no entry for them, so
// `fixed_bindings` binds their keys.
actions!(
    bachuc,
    [
        Quit,
        Hide,
        HideOthers,
        ShowAll,
        Minimize,
        Zoom,
        ToggleFullScreen,
        About
    ]
);

/// Key context of the root view. The shortcuts apply anywhere inside it.
pub const SHELL_CONTEXT: &str = "BachucShell";
/// Key context of an agent pane: find, interrupt, and the mode cycle apply only there.
pub const AGENT_CONTEXT: &str = "BachucAgent";
/// Added to the root view while Settings shows, so Escape leaves Settings as in Electron.
pub const SETTINGS_CONTEXT: &str = "BachucSettings";

/// The whole-UI zoom actions: one step in, one step out, and back to 100%.
pub const ZOOM_IN: &str = "view.zoom.in";
pub const ZOOM_OUT: &str = "view.zoom.out";
pub const ZOOM_RESET: &str = "view.zoom.reset";

/// Actions bound in the agent pane only. Escape and Shift+Tab mean something else in a
/// terminal or a text field, so they never bind on the whole window.
const AGENT_ACTIONS: [&str; 2] = ["agent.interrupt", "message-input.mode-cycle"];

/// Actions on a bare key, such as `?`, that must type in a text field or the terminal. Electron
/// binds them with `focusScope: "other"`.
const OUTSIDE_TEXT_ACTIONS: [&str; 1] = ["shortcuts.dialog.toggle"];

/// The key contexts that take typed text: gpui-kit inputs (the composer, the command center,
/// and settings fields) and the terminal.
pub(crate) const TEXT_CONTEXTS: [&str; 5] =
    ["Input", "Terminal", "NumberInput", "TimeField", "Combobox"];

/// The shell context with no text field or terminal focused, as a binding predicate. GPUI's
/// `!` tests every context of the focus path, so a focused composer in the shell fails it.
pub fn outside_text_context() -> &'static str {
    static CONTEXT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CONTEXT.get_or_init(|| {
        TEXT_CONTEXTS
            .iter()
            .fold(SHELL_CONTEXT.to_string(), |predicate, context| {
                format!("{predicate} && !{context}")
            })
    })
}

/// The key context a shortcut action binds in.
pub fn binding_context(action: &str) -> &'static str {
    if AGENT_ACTIONS.contains(&action) {
        AGENT_CONTEXT
    } else if OUTSIDE_TEXT_ACTIONS.contains(&action) {
        outside_text_context()
    } else {
        SHELL_CONTEXT
    }
}

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
        let context = binding_context(&shortcut.action);
        for keys in keystrokes {
            let alias = (shortcut.action == ZOOM_IN)
                .then(|| plus_alias(&keys))
                .flatten();
            specs.push(BindingSpec {
                keys,
                action: Bound::Run(shortcut.action.clone()),
                context,
            });
            if let Some(keys) = alias {
                specs.push(BindingSpec {
                    keys,
                    action: Bound::Run(shortcut.action.clone()),
                    context,
                });
            }
        }
    }
    specs
}

/// The `+` form of a binding whose last key is `=` without Shift, for example `cmd-+` for
/// `cmd-=`. macOS reports Cmd+Shift+= to GPUI as `cmd-+`, so zoom in answers both, as in
/// browsers.
fn plus_alias(keys: &str) -> Option<String> {
    let (prefix, last) = keys.rsplit_once(' ').unwrap_or(("", keys));
    let modifiers = last.strip_suffix('=')?;
    if modifiers.contains("shift") {
        return None;
    }
    Some(if prefix.is_empty() {
        format!("{modifiers}+")
    } else {
        format!("{prefix} {modifiers}+")
    })
}

/// Bindings that never change, as in Electron: Cmd+F finds in an agent pane, and Escape
/// leaves Settings. A dialog renders outside the root view, so it keeps its own Escape.
pub fn fixed_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-f", FindInTimeline, Some(AGENT_CONTEXT)),
        KeyBinding::new(
            "escape",
            RunAction("settings.toggle".into()),
            Some(SETTINGS_CONTEXT),
        ),
        // The keyboard route to the context menu of the active tab.
        KeyBinding::new("menu", OpenTabMenu, Some(WORKSPACE_CONTEXT)),
        KeyBinding::new("shift-f10", OpenTabMenu, Some(WORKSPACE_CONTEXT)),
        // The standard macOS app keys. No context: they work wherever focus is.
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]
}

/// The menu bar, as in Electron's `features/menu.ts` without Reload, Force Reload, and DevTools.
/// Settings and the zoom items run the shortcut actions, so the shortcut table stays the one
/// source of their keys. The Edit items are OS actions on the gpui-kit input actions: they reach
/// the focused input and grey out when none handles them.
pub fn app_menus() -> Vec<Menu> {
    let run = |name: &'static str, id: &str| MenuItem::action(name, RunAction(id.into()));
    vec![
        Menu::new("Bachuc GPUI").items([
            MenuItem::action("About Bachuc GPUI", About),
            MenuItem::separator(),
            run("Settings…", "settings.toggle"),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Hide Bachuc GPUI", Hide),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::action("Show All", ShowAll),
            MenuItem::separator(),
            MenuItem::action("Quit Bachuc GPUI", Quit),
        ]),
        Menu::new("Edit").items([
            MenuItem::os_action("Undo", input::Undo, OsAction::Undo),
            MenuItem::os_action("Redo", input::Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("Cut", input::Cut, OsAction::Cut),
            MenuItem::os_action("Copy", input::Copy, OsAction::Copy),
            MenuItem::os_action("Paste", input::Paste, OsAction::Paste),
            MenuItem::os_action("Select All", input::SelectAll, OsAction::SelectAll),
        ]),
        Menu::new("View").items([
            run("Zoom In", ZOOM_IN),
            run("Zoom Out", ZOOM_OUT),
            run("Actual Size", ZOOM_RESET),
            MenuItem::separator(),
            MenuItem::action("Enter Full Screen", ToggleFullScreen),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", Minimize),
            MenuItem::action("Zoom", Zoom),
        ]),
    ]
}

/// The root view's key context while Settings shows: the shell shortcuts still apply.
pub fn settings_key_context() -> String {
    format!("{SHELL_CONTEXT} {SETTINGS_CONTEXT}")
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
    use gpui_kit::{KeyContext, Keystroke};

    #[test]
    fn every_assigned_shortcut_binds_and_digits_bind_nine_tabs() {
        let shortcuts = default_shortcuts();
        let specs = binding_specs(&shortcuts);
        // Nine digits, one binding for every other action, and Cmd++ for zoom in.
        assert_eq!(specs.len(), 9 + shortcuts.len() - 1 + 1);
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
    fn escape_and_the_settings_shortcut_leave_settings_only_while_it_shows() {
        let escape = fixed_bindings()
            .into_iter()
            .find(|binding| {
                binding.match_keystrokes(&[Keystroke::parse("escape").unwrap()]) == Some(false)
            })
            .expect("escape is bound");
        assert!(
            escape
                .action()
                .partial_eq(&RunAction("settings.toggle".into()))
        );
        let predicate = escape.predicate().expect("scoped");
        let settings = KeyContext::try_from(settings_key_context().as_str()).unwrap();
        let shell = KeyContext::try_from(SHELL_CONTEXT).unwrap();
        assert!(predicate.eval(std::slice::from_ref(&settings)));
        assert!(!predicate.eval(std::slice::from_ref(&shell)));

        // Cmd+, keeps working while Settings shows.
        let toggle = binding_specs(&default_shortcuts())
            .into_iter()
            .find(|spec| spec.action == Bound::Run("settings.toggle".into()))
            .expect("bound")
            .binding();
        assert!(toggle.predicate().expect("scoped").eval(&[settings]));
    }

    #[test]
    fn every_focus_target_in_this_crate_has_a_role() {
        // GPUI logs "has no accessibility node" when a focused element has an id but no role.
        for (name, source) in [
            ("shell.rs", include_str!("shell.rs")),
            ("panes.rs", include_str!("panes.rs")),
            ("chrome.rs", include_str!("chrome.rs")),
            ("new_workspace.rs", include_str!("new_workspace.rs")),
            ("workspace_draft.rs", include_str!("workspace_draft.rs")),
            ("plugin_views.rs", include_str!("plugin_views.rs")),
            ("plugins.rs", include_str!("plugins.rs")),
            ("dock.rs", include_str!("dock.rs")),
            ("main.rs", include_str!("main.rs")),
            ("state.rs", include_str!("state.rs")),
        ] {
            let mut element = String::new();
            for line in source.lines() {
                let line = line.trim();
                if line.contains("div()") {
                    element.clear();
                }
                element.push_str(line);
                if line.contains(".track_focus(") {
                    assert!(
                        element.contains(".id(") && element.contains(".role("),
                        "{name}: a .track_focus element lacks .id() or .role(): {line}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_menu_key_and_shift_f10_open_the_tab_menu_inside_the_workspace() {
        let workspace = KeyContext::try_from(WORKSPACE_CONTEXT).unwrap();
        let shell = KeyContext::try_from(SHELL_CONTEXT).unwrap();
        for keys in ["menu", "shift-f10"] {
            let binding = fixed_bindings()
                .into_iter()
                .find(|binding| {
                    binding.match_keystrokes(&[Keystroke::parse(keys).unwrap()]) == Some(false)
                        && binding.action().partial_eq(&OpenTabMenu)
                })
                .unwrap_or_else(|| panic!("{keys} is bound to OpenTabMenu"));
            let predicate = binding.predicate().expect("scoped");
            assert!(predicate.eval(std::slice::from_ref(&workspace)));
            assert!(!predicate.eval(std::slice::from_ref(&shell)));
        }
    }

    #[test]
    fn question_mark_runs_only_outside_text_fields_and_the_terminal() {
        let spec = binding_specs(&default_shortcuts())
            .into_iter()
            .find(|spec| spec.action == Bound::Run("shortcuts.dialog.toggle".into()))
            .expect("bound");
        assert_eq!(spec.keys, "?");
        let predicate = spec.binding().predicate().expect("scoped");
        let path = |contexts: &[&str]| -> Vec<KeyContext> {
            contexts
                .iter()
                .map(|context| KeyContext::try_from(*context).unwrap())
                .collect()
        };
        let allowed = [
            vec![SHELL_CONTEXT],
            vec![SHELL_CONTEXT, WORKSPACE_CONTEXT, AGENT_CONTEXT, "Timeline"],
        ];
        for contexts in allowed {
            assert!(
                predicate.depth_of(&path(&contexts)).is_some(),
                "{contexts:?}"
            );
        }
        let settings = settings_key_context();
        let typing = [
            // The composer of an agent pane.
            vec![
                SHELL_CONTEXT,
                WORKSPACE_CONTEXT,
                AGENT_CONTEXT,
                "Input mode=editor",
            ],
            vec![SHELL_CONTEXT, WORKSPACE_CONTEXT, "Terminal"],
            // The command center input.
            vec![SHELL_CONTEXT, "Input"],
            vec![settings.as_str(), "Input"],
            vec![settings.as_str(), "NumberInput", "Input"],
        ];
        for contexts in typing {
            assert!(
                predicate.depth_of(&path(&contexts)).is_none(),
                "{contexts:?}"
            );
        }
        // Every other shortcut keeps its context.
        assert_eq!(binding_context("command-center.toggle"), SHELL_CONTEXT);
        assert_eq!(binding_context("agent.interrupt"), AGENT_CONTEXT);
    }

    /// The pieces of a binding context predicate that name one context, for matching a fixed
    /// binding against a shortcut binding.
    fn context_path(context: &str) -> Vec<KeyContext> {
        let first = context.split(" && ").next().unwrap_or(context);
        vec![KeyContext::try_from(first).unwrap()]
    }

    #[test]
    fn no_fixed_binding_repeats_a_shortcut_table_keystroke_in_the_same_context() {
        let specs = binding_specs(&default_shortcuts());
        let fixed = fixed_bindings();
        assert!(!fixed.is_empty() && !specs.is_empty());
        for binding in &fixed {
            for spec in &specs {
                let typed: Vec<Keystroke> = spec
                    .keys
                    .split(' ')
                    .map(|keys| Keystroke::parse(keys).unwrap())
                    .collect();
                if binding.match_keystrokes(&typed) != Some(false) {
                    continue;
                }
                // The same keys may bind in another context, such as Escape in Settings
                // against Escape in an agent pane.
                let shares_context = binding
                    .predicate()
                    .is_none_or(|predicate| predicate.eval(&context_path(spec.context)));
                assert!(
                    !shares_context,
                    "{} is bound by both the fixed bindings and {:?}",
                    spec.keys, spec.action
                );
            }
        }
    }

    #[test]
    fn the_app_keys_bind_their_actions_in_every_context() {
        let fixed = fixed_bindings();
        let bound = |keys: &str, action: &dyn gpui_kit::Action| {
            fixed.iter().any(|binding| {
                binding.match_keystrokes(&[Keystroke::parse(keys).unwrap()]) == Some(false)
                    && binding.action().partial_eq(action)
                    && binding.predicate().is_none()
            })
        };
        assert!(bound("cmd-q", &Quit));
        assert!(bound("cmd-h", &Hide));
        assert!(bound("alt-cmd-h", &HideOthers));
        assert!(bound("cmd-m", &Minimize));
    }

    #[test]
    fn the_menu_bar_has_the_four_menus_without_the_electron_only_items() {
        let menus = app_menus();
        let names: Vec<_> = menus.iter().map(|menu| menu.name.to_string()).collect();
        assert_eq!(names, ["Bachuc GPUI", "Edit", "View", "Window"]);
        let mut labels = Vec::new();
        for menu in &menus {
            for item in &menu.items {
                if let MenuItem::Action { name, .. } = item {
                    labels.push(name.to_string());
                }
            }
        }
        for expected in [
            "Settings…",
            "Undo",
            "Redo",
            "Cut",
            "Copy",
            "Paste",
            "Select All",
            "Zoom In",
            "Zoom Out",
            "Actual Size",
            "Minimize",
            "Zoom",
        ] {
            assert!(labels.iter().any(|label| label == expected), "{expected}");
        }
        for left_out in ["Reload", "Force Reload", "Toggle Developer Tools"] {
            assert!(!labels.iter().any(|label| label == left_out), "{left_out}");
        }
    }

    #[test]
    fn the_menu_runs_the_shortcut_actions_that_the_table_binds() {
        // A menu item shows the keys of the keymap binding of its action, so these ids must
        // be the ones the shortcut table binds.
        let specs = binding_specs(&default_shortcuts());
        for id in ["settings.toggle", ZOOM_IN, ZOOM_OUT, ZOOM_RESET] {
            assert!(
                specs
                    .iter()
                    .any(|spec| spec.action == Bound::Run(id.into())),
                "{id}"
            );
        }
    }

    #[test]
    fn zoom_in_binds_cmd_equal_and_cmd_plus() {
        let specs = binding_specs(&default_shortcuts());
        let keys = |action: &str| -> Vec<String> {
            specs
                .iter()
                .filter(|spec| spec.action == Bound::Run(action.into()))
                .map(|spec| spec.keys.clone())
                .collect()
        };
        assert_eq!(keys(ZOOM_IN), ["cmd-=", "cmd-+"]);
        assert_eq!(keys(ZOOM_OUT), ["cmd--"]);
        assert_eq!(keys(ZOOM_RESET), ["cmd-0"]);
        for spec in &specs {
            spec.binding();
        }
        assert_eq!(plus_alias("cmd-k cmd-="), Some("cmd-k cmd-+".into()));
        assert_eq!(plus_alias("shift-cmd-="), None);
        assert_eq!(plus_alias("cmd-p"), None);
    }

    #[test]
    fn an_empty_zoom_override_binds_no_key() {
        let mut shortcuts = default_shortcuts();
        for shortcut in &mut shortcuts {
            if [ZOOM_IN, ZOOM_OUT, ZOOM_RESET].contains(&shortcut.action.as_str()) {
                shortcut.chord = Some(String::new());
            }
        }
        let specs = binding_specs(&shortcuts);
        assert!(specs.iter().all(|spec| !spec.keys.is_empty()));
        assert!(specs.iter().all(|spec| {
            ![ZOOM_IN, ZOOM_OUT, ZOOM_RESET]
                .iter()
                .any(|id| spec.action == Bound::Run((*id).into()))
        }));
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
