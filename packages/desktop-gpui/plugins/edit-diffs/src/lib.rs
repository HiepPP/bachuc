//! `bachuc-plugin-edit-diffs`: the native client of the edit-diffs plugin
//! (`hiep-plugins/plugins/edit-diffs/index.client.tsx`, ADR-0045). It turns edit and write tool
//! calls into `edit-diff` rows, draws them, and adds the "Edit diffs" settings screen and the
//! thread toggle. The server part stays in TypeScript; this crate calls it only through
//! `edit-diffs.locate` and the `display` settings RPCs.

mod diff;
mod edit_item;
mod highlight;
mod row;
mod settings_view;
mod state;

use crate::edit_item::{EDIT_DIFFS_MODE_LABEL, edit_diffs_off, to_edit_row, toggled_mode};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    CommandCenterItem, CommandContext, CommandInput, Contributions, MessageLevel, NativePlugin,
    PluginContext, PluginTimelineItem, SettingsScreen, TimelineRenderer, TimelineTransformInput,
    TimelineTransformResult, TimelineTransformer,
};
use gpui_kit::{App, AppContext as _};
use std::collections::BTreeMap;
use std::rc::Rc;

const PLUGIN_ID: &str = "edit-diffs";
const ITEM_KIND: &str = "edit-diff";
const ITEM_VERSION: u32 = 1;
const SETTINGS_ID: &str = "display";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(EditDiffs)
}

struct EditDiffs;

impl NativePlugin for EditDiffs {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        // One settings record serves every row and the screen.
        let settings = ctx.settings(SETTINGS_ID, cx);
        let rows = Rc::new(row::Rows::new(ctx.clone(), settings.clone()));

        contributions.add_timeline_transformer(TimelineTransformer {
            id: "edit-diff",
            item_type: "tool_call",
            transform: Rc::new(transform),
        });
        contributions.add_timeline_renderer(TimelineRenderer {
            kind: ITEM_KIND,
            version: ITEM_VERSION,
            view: Rc::new(move |input, _window, cx| row::build(&rows, input, cx)),
        });
        contributions.add_settings_screen(SettingsScreen {
            id: SETTINGS_ID,
            title: "Edit diffs".into(),
            icon: IconName::PencilOff,
            view: Rc::new(move |(), _window, cx| {
                let settings = settings.clone();
                cx.new(|cx| settings_view::DisplaySettingsView::new(settings, cx))
                    .into()
            }),
        });
        contributions.add_command_item(CommandCenterItem {
            id: "toggle-thread",
            title: "Turn edit diffs on or off in this thread".into(),
            icon: IconName::PencilOff,
            keywords: vec!["edit".into(), "diff".into(), "thread".into()],
            context: CommandContext::Agent,
            visible: None,
            on_select: Rc::new(move |input, _window, cx| toggle_thread(&ctx, input, cx)),
        });
    }
}

/// The `edit-diff` transformer: `None` for a thread with edit diffs off and for a call that is
/// not an edit or a write; else one row that replaces the call.
fn transform(input: &TimelineTransformInput<'_>) -> Option<TimelineTransformResult> {
    if edit_diffs_off(input.labels) {
        return None;
    }
    let data = to_edit_row(input.item)?;
    // The call id keeps a row's state across the views the timeline builds while it streams.
    let id = input.item["callId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_string);
    Some(TimelineTransformResult {
        items: vec![PluginTimelineItem {
            id,
            kind: ITEM_KIND.into(),
            version: ITEM_VERSION,
            data,
        }],
        source: None,
    })
}

/// Sets `edit-diffs.mode` to `on` when it is `off`, else `off`. A failure shows as an error
/// message, as Electron toasts a failed command.
fn toggle_thread(ctx: &PluginContext, input: CommandInput, cx: &mut App) {
    let (Some(agent_id), Some(client)) = (input.agent_id, ctx.client(cx)) else {
        return;
    };
    let labels = client.read(cx).agent_labels(&agent_id).unwrap_or_default();
    let mode = BTreeMap::from([(
        EDIT_DIFFS_MODE_LABEL.to_string(),
        toggled_mode(&labels).to_string(),
    )]);
    let saved = client.update(cx, |client, cx| {
        client.set_agent_labels(&agent_id, mode, cx)
    });
    let ctx = ctx.clone();
    cx.spawn(async move |cx| {
        if let Err(error) = saved.await {
            cx.update(|cx| ctx.show_message(format!("{error:#}").into(), MessageLevel::Error, cx));
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_plugin_host::TimelinePhase;
    use serde_json::{Value, json};

    fn run(labels: &BTreeMap<String, String>, item: &Value) -> Option<TimelineTransformResult> {
        transform(&TimelineTransformInput {
            agent_id: "a1",
            labels,
            item,
            phase: TimelinePhase::Complete,
        })
    }

    fn edit_call() -> Value {
        json!({
            "type": "tool_call",
            "callId": "call-7",
            "name": "Edit",
            "status": "running",
            "detail": {"type": "edit", "filePath": "/w/a.rs", "unifiedDiff": "@@\n-a\n+b\n"},
            "error": null,
        })
    }

    #[test]
    fn an_off_thread_keeps_its_calls() {
        let off = BTreeMap::from([(EDIT_DIFFS_MODE_LABEL.to_string(), "off".to_string())]);
        assert_eq!(run(&off, &edit_call()), None);
    }

    #[test]
    fn a_read_call_keeps_its_row() {
        let item = json!({
            "type": "tool_call", "callId": "c", "status": "completed",
            "detail": {"type": "read", "filePath": "/a"},
        });
        assert_eq!(run(&BTreeMap::new(), &item), None);
    }

    #[test]
    fn an_edit_call_becomes_one_edit_diff_item_that_hides_the_call() {
        let on = BTreeMap::from([(EDIT_DIFFS_MODE_LABEL.to_string(), "on".to_string())]);
        let result = run(&on, &edit_call()).unwrap();
        assert_eq!(result.source, None);
        assert_eq!(
            result.items,
            vec![PluginTimelineItem {
                id: Some("call-7".into()),
                kind: "edit-diff".into(),
                version: 1,
                data: json!({
                    "action": "edit",
                    "filePath": "/w/a.rs",
                    "unifiedDiff": "@@\n-a\n+b\n",
                    "status": "running",
                }),
            }]
        );
        // Without a call id the host gives the item its own id.
        let mut item = edit_call();
        item["callId"] = Value::Null;
        assert_eq!(run(&BTreeMap::new(), &item).unwrap().items[0].id, None);
    }
}
