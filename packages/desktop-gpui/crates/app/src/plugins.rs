//! The native plugin clients the app links (ADR-0045), and the pure rules of their app-level
//! contributions: workspace panel location and tab identity, surface heading, shortcut match,
//! header subtitle, and the New workspace side panel.

use bachuc_command::{Shortcut, gpui_keystrokes};
use bachuc_plugin_host::{HeaderSubtitleInput, NativePlugin, PanelContext, PanelLocation};

/// Every plugin crate the app links, in registration order. Each exports
/// `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>`.
pub fn registered_plugins() -> Vec<Box<dyn NativePlugin>> {
    Vec::new()
}

/// `resolvePluginPanelOpenLocation`: the requested location, else `Workspace` when listed, else
/// the first. A panel with no locations lists `Workspace`.
pub fn panel_location(
    panel_id: &str,
    locations: &[PanelLocation],
    requested: Option<PanelLocation>,
) -> Result<PanelLocation, String> {
    let listed: &[PanelLocation] = if locations.is_empty() {
        &[PanelLocation::Workspace]
    } else {
        locations
    };
    let location = requested.or_else(|| {
        if listed.contains(&PanelLocation::Workspace) {
            Some(PanelLocation::Workspace)
        } else {
            listed.first().copied()
        }
    });
    match location {
        Some(location) if listed.contains(&location) => Ok(location),
        _ => Err(format!(
            "Workspace panel {panel_id} does not support {} location",
            match requested {
                Some(PanelLocation::Workspace) => "workspace",
                Some(PanelLocation::Explorer) => "explorer",
                None => "any",
            }
        )),
    }
}

/// The tab identity of a panel: `plugin:<plugin>/<panel>`, plus `/<agent>` for an agent panel.
pub fn panel_identity(
    plugin_id: &str,
    panel_id: &str,
    context: PanelContext,
    agent_id: Option<&str>,
) -> String {
    match (context, agent_id) {
        (PanelContext::Agent, Some(agent_id)) => {
            format!("plugin:{plugin_id}/{panel_id}/{agent_id}")
        }
        _ => format!("plugin:{plugin_id}/{panel_id}"),
    }
}

/// `pluginSurfaceHeading`: the surface title, else its id, else the plugin id, else "Plugin".
pub fn surface_heading(surface: Option<(Option<&str>, &str)>, plugin_id: &str) -> String {
    match surface {
        Some((Some(title), _)) => title.to_string(),
        Some((None, id)) => id.to_string(),
        None if !plugin_id.is_empty() => plugin_id.to_string(),
        None => "Plugin".to_string(),
    }
}

/// The text of a keystroke in the `gpui_keystrokes` form, for example `cmd-shift-g`.
pub fn keystroke_text(ctrl: bool, alt: bool, shift: bool, cmd: bool, key: &str) -> String {
    let mut text = String::new();
    for (on, name) in [(ctrl, "ctrl"), (alt, "alt"), (shift, "shift"), (cmd, "cmd")] {
        if on {
            text.push_str(name);
            text.push('-');
        }
    }
    text.push_str(&key.to_ascii_lowercase());
    text
}

/// Whether a host shortcut takes the key: the first combo of every assigned chord, whatever its
/// context. An unassigned or empty chord takes none.
pub fn host_takes(host: &[Shortcut], pressed: &str) -> bool {
    host.iter().any(|shortcut| {
        shortcut
            .chord
            .as_deref()
            .and_then(|chord| chord.split_whitespace().next())
            .is_some_and(|first| gpui_keystrokes(first).iter().any(|keys| keys == pressed))
    })
}

/// `resolvePluginShortcut`: the index of the first plugin combo that matches, when no host
/// shortcut uses the key. `Mod` means Cmd. A multi-key plugin combo never matches.
pub fn plugin_shortcut(host: &[Shortcut], combos: &[&str], pressed: &str) -> Option<usize> {
    if host_takes(host, pressed) {
        return None;
    }
    combos.iter().position(|combo| {
        combo.split_whitespace().count() == 1
            && gpui_keystrokes(combo).iter().any(|keys| keys == pressed)
    })
}

/// The subtitle input when a plugin subtitle replaces the project name: a contribution exists
/// and the workspace has a project id.
pub fn subtitle_input(
    has_contribution: bool,
    workspace_id: &str,
    project_id: &str,
    project_name: &str,
    project_root: &str,
) -> Option<HeaderSubtitleInput> {
    (has_contribution && !project_id.is_empty()).then(|| HeaderSubtitleInput {
        workspace_id: workspace_id.to_string(),
        project_id: project_id.to_string(),
        project_display_name: project_name.to_string(),
        project_root_path: Some(project_root.to_string()).filter(|root| !root.is_empty()),
    })
}

/// The New workspace side panel shows its toggle once a project is chosen and a panel exists.
pub fn side_panel_available(target: Option<&(String, String)>, panels: usize) -> bool {
    target.is_some() && panels > 0
}

/// The panel to show: the one `ShowNewWorkspacePanel` named, else the first.
pub fn side_panel_index(panels: &[(&str, &str)], requested: Option<(&str, &str)>) -> Option<usize> {
    if panels.is_empty() {
        return None;
    }
    let found = requested.and_then(|requested| panels.iter().position(|panel| *panel == requested));
    Some(found.unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::{
        host_takes, keystroke_text, panel_identity, panel_location, plugin_shortcut,
        registered_plugins, side_panel_available, side_panel_index, subtitle_input,
        surface_heading,
    };
    use bachuc_command::{default_shortcuts, gpui_keystrokes};
    use bachuc_plugin_host::{PanelContext, PanelLocation};

    #[test]
    fn no_plugin_is_registered_yet() {
        assert!(registered_plugins().is_empty());
    }

    #[test]
    fn the_panel_location_is_the_request_else_workspace_else_the_first() {
        use PanelLocation::{Explorer, Workspace};
        assert_eq!(
            panel_location("p", &[Workspace, Explorer], None),
            Ok(Workspace)
        );
        assert_eq!(panel_location("p", &[Explorer], None), Ok(Explorer));
        assert_eq!(panel_location("p", &[], None), Ok(Workspace));
        assert_eq!(
            panel_location("p", &[Workspace, Explorer], Some(Explorer)),
            Ok(Explorer)
        );
        assert_eq!(
            panel_location("p", &[Workspace], Some(Explorer)),
            Err("Workspace panel p does not support explorer location".to_string())
        );
    }

    #[test]
    fn an_agent_panel_identity_names_the_agent() {
        assert_eq!(
            panel_identity("notes", "list", PanelContext::Workspace, Some("a1")),
            "plugin:notes/list"
        );
        assert_eq!(
            panel_identity("notes", "list", PanelContext::Agent, Some("a1")),
            "plugin:notes/list/a1"
        );
        // Two opens of the same agent panel give one identity, so one tab.
        assert_eq!(
            panel_identity("notes", "list", PanelContext::Agent, Some("a1")),
            panel_identity("notes", "list", PanelContext::Agent, Some("a1"))
        );
    }

    #[test]
    fn the_surface_title_comes_first_then_the_id_then_the_plugin() {
        assert_eq!(
            surface_heading(Some((Some("Board"), "board")), "wt"),
            "Board"
        );
        assert_eq!(surface_heading(Some((None, "board")), "wt"), "board");
        assert_eq!(surface_heading(None, "wt"), "wt");
        assert_eq!(surface_heading(None, ""), "Plugin");
    }

    #[test]
    fn mod_means_cmd() {
        assert_eq!(gpui_keystrokes("Mod+G"), ["cmd-g"]);
        assert_eq!(keystroke_text(false, false, false, true, "G"), "cmd-g");
        assert_eq!(
            plugin_shortcut(
                &[],
                &["Mod+G"],
                &keystroke_text(false, false, false, true, "g")
            ),
            Some(0)
        );
    }

    #[test]
    fn a_host_key_wins_over_a_plugin_and_an_unassigned_host_key_takes_nothing() {
        let host = default_shortcuts();
        // Cmd+K is the command center.
        assert_eq!(plugin_shortcut(&host, &["Mod+K"], "cmd-k"), None);
        // Cmd+Shift+G is Changes by default; unassigned, a plugin may use it.
        let mut unassigned = default_shortcuts();
        for shortcut in &mut unassigned {
            if shortcut.chord.as_deref() == Some("Cmd+Shift+G") {
                shortcut.chord = None;
            }
        }
        assert_eq!(
            plugin_shortcut(&host, &["Mod+Shift+G"], "shift-cmd-g"),
            None
        );
        assert_eq!(
            plugin_shortcut(&unassigned, &["Mod+Shift+G"], "shift-cmd-g"),
            Some(0)
        );
        // A free key goes to the first plugin with the combo.
        assert_eq!(
            plugin_shortcut(&host, &["Mod+J", "Mod+G", "Mod+G"], "cmd-g"),
            Some(1)
        );
        // An empty chord takes no key.
        let mut empty = default_shortcuts();
        empty[0].chord = Some(String::new());
        assert!(!host_takes(&empty, "cmd-k"));
    }

    #[test]
    fn no_default_host_chord_ever_reaches_a_plugin() {
        // The anti-goal tripwire: a plugin that registers every host combo gets no key.
        let host = default_shortcuts();
        for shortcut in &host {
            let chord = shortcut.chord.as_deref().expect("assigned");
            let first = chord.split_whitespace().next().expect("combo");
            for pressed in gpui_keystrokes(first) {
                assert_eq!(
                    plugin_shortcut(&host, &[first], &pressed),
                    None,
                    "{chord} reached a plugin"
                );
            }
        }
    }

    #[test]
    fn the_subtitle_replaces_the_project_name_only_with_a_project_id() {
        assert!(subtitle_input(true, "w1", "", "Bachuc", "/repo").is_none());
        assert!(subtitle_input(false, "w1", "p1", "Bachuc", "/repo").is_none());
        let input = subtitle_input(true, "w1", "p1", "Bachuc", "/repo").expect("input");
        assert_eq!(input.project_display_name, "Bachuc");
        assert_eq!(input.project_root_path.as_deref(), Some("/repo"));
    }

    #[test]
    fn the_side_panel_needs_a_project_and_a_panel() {
        let target = ("p1".to_string(), "/repo".to_string());
        assert!(side_panel_available(Some(&target), 1));
        assert!(!side_panel_available(None, 1));
        assert!(!side_panel_available(Some(&target), 0));
        let panels = [("a", "one"), ("b", "two")];
        assert_eq!(side_panel_index(&panels, Some(("b", "two"))), Some(1));
        assert_eq!(side_panel_index(&panels, Some(("b", "gone"))), Some(0));
        assert_eq!(side_panel_index(&panels, None), Some(0));
        assert_eq!(side_panel_index(&[], None), None);
    }
}
