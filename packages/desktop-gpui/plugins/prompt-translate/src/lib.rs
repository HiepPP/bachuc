//! `bachuc-plugin-prompt-translate`: the native client of the prompt-translate plugin
//! (`hiep-plugins/plugins/prompt-translate/index.client.tsx`, ADR-0045). It adds the EN or VI
//! gốc note under each user prompt, the send interceptor that rewrites drafts as English, the
//! Caveman mode pill, and the "Prompt translate" settings screen. The server part stays in
//! TypeScript; this crate calls it only through its `translate.*` RPCs and the settings record
//! `translate`.

mod intercept;
mod note_view;
mod notes;
mod prefs;
mod rpc;
mod settings;
mod settings_view;
mod shared;
mod state;
mod vietnamese;

use crate::intercept::{Plan, outcome, plan, transform};
use crate::rpc::{ENHANCE, enhance_input, parse_prompt};
use crate::settings_view::PromptTranslateSettingsView;
use crate::shared::{Shared, Watcher};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    ComposerInterceptor, ComposerPill, Contributions, InterceptInput, InterceptOutcome,
    MessageLevel, NativePlugin, PillPlacement, PluginContext, SettingsScreen, ShowOnDraft,
    TimelineRenderer, TimelineTransformer,
};
use gpui_kit::{App, AppContext as _, Task};
use std::rc::Rc;

pub(crate) const PLUGIN_ID: &str = "prompt-translate";
const SETTINGS_ID: &str = "translate";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(PromptTranslate)
}

struct PromptTranslate;

impl NativePlugin for PromptTranslate {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        // One settings record, one note cache, and one mode store serve every contribution.
        let settings = ctx.settings(SETTINGS_ID, cx);
        let shared = Shared::new(ctx, settings.clone());
        // The pill closure holds the watcher, so deactivation drops its subscriptions.
        let watcher = {
            let shared = shared.clone();
            cx.new(|cx| Watcher::new(shared, cx))
        };

        // The native bubble stays; the EN or VI gốc note renders under it.
        contributions.add_timeline_transformer(TimelineTransformer {
            id: "prompt-note",
            item_type: "user_message",
            transform: {
                let shared = shared.clone();
                Rc::new(move |input| {
                    let translate = shared.current.borrow().values.translate;
                    transform(input, translate)
                })
            },
        });
        contributions.add_timeline_renderer(TimelineRenderer {
            kind: intercept::NOTE_KIND,
            version: intercept::NOTE_VERSION,
            view: {
                let shared = shared.clone();
                Rc::new(move |input, _window, cx| note_view::build(&shared, input, cx))
            },
        });
        contributions.add_composer_interceptor(ComposerInterceptor {
            id: "rewrite-english",
            intercept: {
                let shared = shared.clone();
                Rc::new(move |input, cx| rewrite(&shared, input, cx))
            },
        });
        contributions.add_composer_pill(ComposerPill {
            id: "prompt-prefs",
            placement: PillPlacement::Toolbar,
            show_on_draft: ShowOnDraft::Yes,
            workspace_id: None,
            agent_id: None,
            state: Rc::new(move |target, cx| {
                let _keep = &watcher;
                shared.pill_state(target, cx)
            }),
        });
        contributions.add_settings_screen(SettingsScreen {
            id: SETTINGS_ID,
            title: "Prompt translate".into(),
            icon: IconName::Languages,
            view: Rc::new(move |(), window, cx| {
                let settings = settings.clone();
                cx.new(|cx| PromptTranslateSettingsView::new(settings, window, cx))
                    .into()
            }),
        });
    }
}

/// The send interceptor. An enhance error shows its text and cancels the send, which keeps the
/// draft.
fn rewrite(shared: &Rc<Shared>, input: InterceptInput, cx: &mut App) -> Task<InterceptOutcome> {
    let settings = shared.current.borrow().values.clone();
    if plan(&settings, &input.text, input.send_key) == Plan::Keep {
        return Task::ready(InterceptOutcome::Keep);
    }
    let answer = shared.ctx.rpc(ENHANCE, enhance_input(&input.text), cx);
    let ctx = shared.ctx.clone();
    cx.spawn(async move |cx| {
        let (result, message) = outcome(answer.await.and_then(|value| parse_prompt(&value)));
        if let Some(message) = message {
            cx.update(|cx| ctx.show_message(message.into(), MessageLevel::Error, cx));
        }
        result
    })
}
