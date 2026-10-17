//! `bachuc-plugin-skill-pins`: the native client of the skill-pins plugin
//! (`hiep-plugins/plugins/skill-pins/index.client.tsx`, ADR-0045). It adds the Skills pill with
//! its pin menu to agent and draft composers, and the Skill pins settings screen. The server
//! part stays in TypeScript; this crate calls it only through `skill-pins.read` and
//! `skill-pins.write` and the settings record `skill-pins`.

mod catalog;
mod menu;
mod pins;
mod settings_view;

use crate::catalog::pill_label;
use crate::menu::SkillsMenu;
use crate::pins::{Pins, Watcher};
use crate::settings_view::SkillPinsSettingsView;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    ComposerPill, Contributions, NativePlugin, PillBehavior, PillPlacement, PillState,
    PluginContext, SettingsScreen, ShowOnDraft,
};
use gpui_kit::{App, AppContext as _};
use std::rc::Rc;

pub(crate) const PLUGIN_ID: &str = "skill-pins";
pub(crate) const SETTINGS_ID: &str = "skill-pins";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(SkillPins)
}

struct SkillPins;

impl NativePlugin for SkillPins {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        // One settings record and one pins store serve the pill, the menu, and the screen.
        let settings = ctx.settings(SETTINGS_ID, cx);
        let pins = Pins::new(ctx, settings.clone());
        // The pill closure holds the watcher, so deactivation drops its subscriptions.
        let watcher = {
            let pins = pins.clone();
            cx.new(|cx| Watcher::new(pins, cx))
        };

        contributions.add_settings_screen(SettingsScreen {
            id: SETTINGS_ID,
            title: "Skill pins".into(),
            icon: IconName::Pin,
            view: Rc::new(move |(), _window, cx| {
                let settings = settings.clone();
                cx.new(|cx| SkillPinsSettingsView::new(settings, cx)).into()
            }),
        });

        // One pill for every agent composer and for the New workspace draft.
        contributions.add_composer_pill(ComposerPill {
            id: "skills",
            placement: PillPlacement::Toolbar,
            show_on_draft: ShowOnDraft::Yes,
            workspace_id: None,
            agent_id: None,
            state: Rc::new(move |target, cx| {
                let _keep = &watcher;
                if target.draft.is_none() && target.agent_id.is_none() {
                    return None;
                }
                // Unknown pins show `Skills`, as Electron does.
                let skills = pins.current(target, cx).unwrap_or_default();
                let pins = pins.clone();
                Some(PillState {
                    title: "Pinned skills".into(),
                    label: Some(pill_label(&skills).into()),
                    icon: None,
                    color: None,
                    disabled: false,
                    behavior: PillBehavior::Popover {
                        flush: true,
                        content: Rc::new(move |target, _close, _window, cx| {
                            let pins = pins.clone();
                            cx.new(|cx| SkillsMenu::new(pins, target, cx)).into()
                        }),
                    },
                })
            }),
        });
    }
}
