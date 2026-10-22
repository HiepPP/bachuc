//! `bachuc-plugin-selection-translate`: the native client of the selection-translate plugin
//! (`hiep-plugins/plugins/selection-translate`, ADR-0062, ADR-0063). It adds the `Translate` result
//! action on selected assistant text and the "Selection translate" settings screen. The server
//! part stays in TypeScript; this crate calls it only through its `selection.translate` RPC and
//! the settings record `translate`.

mod request;
mod settings;
mod settings_view;

use crate::request::{TRANSLATE, parse_translation, request_input};
use crate::settings_view::SelectionTranslateSettingsView;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    AssistantSelection, Contributions, NativePlugin, PluginContext, SelectionResultAction,
    SettingsScreen,
};
use gpui_kit::{App, AppContext as _, Task};
use std::rc::Rc;

const PLUGIN_ID: &str = "selection-translate";
const SETTINGS_ID: &str = "translate";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(SelectionTranslate)
}

struct SelectionTranslate;

impl NativePlugin for SelectionTranslate {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        let settings = ctx.settings(SETTINGS_ID, cx);
        contributions.add_selection_result_action(SelectionResultAction {
            id: "translate",
            title: "Translate".into(),
            loading: "Đang dịch...".into(),
            run: Rc::new(move |selection, cx| translate(&ctx, selection, cx)),
        });
        contributions.add_settings_screen(SettingsScreen {
            id: SETTINGS_ID,
            title: "Selection translate".into(),
            icon: IconName::Languages,
            view: Rc::new(move |(), window, cx| {
                let settings = settings.clone();
                cx.new(|cx| SelectionTranslateSettingsView::new(settings, window, cx))
                    .into()
            }),
        });
    }
}

/// The card text: the Vietnamese translation, or the error the card shows.
fn translate(
    ctx: &PluginContext,
    selection: AssistantSelection,
    cx: &mut App,
) -> Task<anyhow::Result<String>> {
    let input = match request_input(&selection.text) {
        Ok(input) => input,
        Err(error) => return Task::ready(Err(error)),
    };
    let answer = ctx.rpc(TRANSLATE, input, cx);
    cx.spawn(async move |_cx| parse_translation(&answer.await?))
}
