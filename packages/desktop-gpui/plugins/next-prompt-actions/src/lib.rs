//! `bachuc-plugin-next-prompt-actions`: the native client of the next-prompt-actions plugin
//! (`hiep-plugins/plugins/next-prompt-actions/index.client.tsx`, ADR-0045). It folds a reply's
//! What Next section into an action panel, adds the "Next prompt actions" settings screen, and
//! the Jev auto-run command. The server part stays in TypeScript; this crate calls it only
//! through its `prompts.*` RPCs and the `send` settings.

mod contracts;
mod model;
mod next_prompts;
mod panel;
mod polling;
mod prompts;
mod rules;
mod section;
mod selection;
mod settings_view;

use crate::contracts::{HOST, HostInfo, INSPECT, Scope, Snapshot, TOGGLE, ToggleInput};
use crate::model::Models;
use crate::panel::Shared;
use crate::rules::command_visible;
use crate::section::{section_intro, split_next_section, split_recap};
use crate::settings_view::SendSettingsView;
use bachuc_client::{Client, ClientEvent};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    CommandCenterItem, CommandContext, CommandInput, Contributions, MessageLevel, NativePlugin,
    PluginContext, PluginTimelineItem, SettingsScreen, SourcePlacement, SourcePosition,
    TimelinePhase, TimelineRenderer, TimelineTransformInput, TimelineTransformResult,
    TimelineTransformer,
};
use gpui_kit::{App, AppContext as _, Context, Entity, Subscription};
use serde_json::json;
use std::cell::Cell;
use std::rc::Rc;

const PLUGIN_ID: &str = "next-prompt-actions";
const ITEM_KIND: &str = "next-prompt-panel";
const ITEM_VERSION: u32 = 1;
const SETTINGS_ID: &str = "send";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(NextPromptActions)
}

struct NextPromptActions;

impl NativePlugin for NextPromptActions {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        // One settings record and one model map serve every panel and the screen.
        let settings = ctx.settings(SETTINGS_ID, cx);
        let shared = Rc::new(Shared {
            ctx: ctx.clone(),
            settings: settings.clone(),
            models: Rc::new(Models::default()),
        });
        let evaluator = Rc::new(Cell::new(false));
        let host_read = cx.new(|cx| HostRead::new(ctx.clone(), evaluator.clone(), cx));

        contributions.add_timeline_transformer(TimelineTransformer {
            id: "next-prompt-panel",
            item_type: "assistant_message",
            transform: Rc::new(transform),
        });
        contributions.add_timeline_renderer(TimelineRenderer {
            kind: ITEM_KIND,
            version: ITEM_VERSION,
            view: Rc::new(move |input, _window, cx| panel::build(&shared, input, cx)),
        });
        contributions.add_settings_screen(SettingsScreen {
            id: SETTINGS_ID,
            title: "Next prompt actions".into(),
            icon: IconName::Send,
            view: Rc::new(move |(), _window, cx| {
                let settings = settings.clone();
                cx.new(|cx| SendSettingsView::new(settings, cx)).into()
            }),
        });
        // Offered only while Jev can judge; re-enabling jev-evaluator needs a reload.
        contributions.add_command_item(CommandCenterItem {
            id: "toggle-auto-run",
            title: "Toggle Jev next-prompt auto-run".into(),
            icon: IconName::Send,
            keywords: vec!["jev".into(), "auto-run".into(), "next prompt".into()],
            context: CommandContext::Agent,
            visible: Some(Rc::new(move |_| {
                // The closure holds the read, so it lives while the contribution does.
                let _ = &host_read;
                evaluator.get()
            })),
            on_select: Rc::new(move |input, _window, cx| toggle_auto_run(&ctx, input, cx)),
        });
    }
}

/// The one `prompts.host` read of an activation. A request sent before `Connected` fails, so
/// it waits for the connection.
struct HostRead {
    read: bool,
    /// Held strong: a subscription holds its emitter weakly.
    _client: Option<Entity<Client>>,
    _subscription: Option<Subscription>,
}

impl HostRead {
    fn new(ctx: PluginContext, evaluator: Rc<Cell<bool>>, cx: &mut Context<Self>) -> Self {
        let client: Option<Entity<Client>> = ctx.client(cx);
        let read = client.as_ref().is_some_and(|c| c.read(cx).is_connected());
        if read {
            read_host(&ctx, &evaluator, cx);
        }
        let subscription = client.as_ref().filter(|_| !read).map(|client| {
            cx.subscribe(
                client,
                move |this: &mut Self, _, event: &ClientEvent, cx| {
                    if matches!(event, ClientEvent::Connected) && !this.read {
                        this.read = true;
                        read_host(&ctx, &evaluator, cx);
                    }
                },
            )
        });
        Self {
            read,
            _client: client,
            _subscription: subscription,
        }
    }
}

fn read_host(ctx: &PluginContext, evaluator: &Rc<Cell<bool>>, cx: &mut App) {
    let read = ctx.rpc_as::<_, HostInfo>(HOST, &json!({}), cx);
    let (ctx, evaluator) = (ctx.clone(), evaluator.clone());
    cx.spawn(async move |cx| {
        let host = read.await.ok();
        evaluator.set(command_visible(host.as_ref()));
        cx.update(|cx| ctx.notify(cx));
    })
    .detach();
}

/// The `next-prompt-panel` transformer: `None` while streaming and for a reply without an
/// eligible What Next section; else the panel, after the reply's text up to the Recap.
fn transform(input: &TimelineTransformInput<'_>) -> Option<TimelineTransformResult> {
    if input.phase != TimelinePhase::Complete {
        return None;
    }
    let split = split_next_section(input.item["text"].as_str()?)?;
    let (before, recap) = split_recap(&split.before);
    let mut data = json!({
        "title": split.title,
        "intro": section_intro(&split.section),
        "section": split.section,
    });
    if let Some(recap) = recap {
        data["recap"] = serde_json::to_value(recap).ok()?;
    }
    Some(TimelineTransformResult {
        items: vec![PluginTimelineItem {
            id: None,
            kind: ITEM_KIND.into(),
            version: ITEM_VERSION,
            data,
        }],
        source: (!before.is_empty()).then_some(SourcePlacement {
            position: SourcePosition::First,
            text: Some(before),
        }),
    })
}

/// Reads the host and the snapshot, then turns auto-run to the other state. A failure shows as
/// an error message, as Electron toasts a failed command.
fn toggle_auto_run(ctx: &PluginContext, input: CommandInput, cx: &mut App) {
    let (Some(agent_id), Some(workspace_id)) = (input.agent_id, input.workspace_id) else {
        return;
    };
    let host = ctx.rpc_as::<_, HostInfo>(HOST, &json!({}), cx);
    let ctx = ctx.clone();
    cx.spawn(async move |cx| {
        let result: anyhow::Result<()> = async {
            let host = host.await?;
            let scope = Scope {
                server_id: host.server_id,
                agent_id,
                workspace_id,
            };
            let inspect = cx.update(|cx| ctx.rpc_as::<_, Snapshot>(INSPECT, &scope, cx));
            let current = inspect.await?;
            let input = ToggleInput {
                scope,
                enabled: !current.enabled,
            };
            let toggle = cx.update(|cx| ctx.rpc_as::<_, Snapshot>(TOGGLE, &input, cx));
            toggle.await?;
            Ok(())
        }
        .await;
        if let Err(error) = result {
            cx.update(|cx| ctx.show_message(format!("{error:#}").into(), MessageLevel::Error, cx));
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::{PLUGIN_ID, plugin, transform};
    use bachuc_plugin_host::{
        SourcePlacement, SourcePosition, TimelinePhase, TimelineTransformInput,
        TimelineTransformResult,
    };
    use serde_json::{Value, json};
    use std::collections::BTreeMap;

    fn run(text: &str, phase: TimelinePhase) -> Option<TimelineTransformResult> {
        let item = json!({"type": "assistant_message", "text": text});
        transform(&TimelineTransformInput {
            agent_id: "a1",
            labels: &BTreeMap::new(),
            item: &item,
            phase,
        })
    }

    const FENCE: &str = "```text\nprompt: Run the tests.\n```";

    #[test]
    fn the_plugin_id_is_next_prompt_actions() {
        assert_eq!(plugin().id(), PLUGIN_ID);
    }

    #[test]
    fn transformer_cases() {
        let reply = format!(
            "Done.\n\n## Recap\n- `main` · not committed\n- Did: Ported it.\n\n## What Next\n\nFirst.\n\n{FENCE}"
        );
        assert_eq!(run(&reply, TimelinePhase::Streaming), None);
        assert_eq!(
            run("Done.\n\n## What Next\n\nNothing.", TimelinePhase::Complete),
            None
        );
        assert_eq!(
            run(
                &format!("```md\n## What Next\n{FENCE}\n```"),
                TimelinePhase::Complete
            ),
            None,
            "a heading inside a fence is ignored"
        );
        let result = run(&reply, TimelinePhase::Complete).unwrap();
        assert_eq!(
            result.source,
            Some(SourcePlacement {
                position: SourcePosition::First,
                text: Some("Done.".into()),
            })
        );
        assert_eq!(result.items.len(), 1);
        let item = &result.items[0];
        assert_eq!(
            (item.kind.as_str(), item.version, item.id.as_ref()),
            ("next-prompt-panel", 1, None)
        );
        assert_eq!(item.data["title"], "What Next");
        assert_eq!(item.data["intro"], "First.");
        assert_eq!(item.data["recap"]["branch"], "`main`");
        assert_eq!(item.data["recap"]["commit"], "not committed");
        assert_eq!(item.data["recap"]["sections"][0]["label"], "Did");
        assert_eq!(item.data["recap"].get("notYet"), None);
        // A reply that is only the section hides its source.
        let only = run(&format!("## Next Steps\n{FENCE}"), TimelinePhase::Complete).unwrap();
        assert_eq!(only.source, None);
        assert_eq!(only.items[0].data.get("recap"), None);
        // Unknown prose before the section stays in the reply.
        let prose = run(
            &format!("## Recap\nSomething else.\n\n## What Next\n{FENCE}"),
            TimelinePhase::Complete,
        )
        .unwrap();
        assert_eq!(
            prose.source.unwrap().text.as_deref(),
            Some("## Recap\nSomething else.")
        );
        assert_eq!(prose.items[0].data.get("recap"), None::<&Value>);
        // A legacy and a five-field Recap fold into the header too.
        let legacy = run(
            &format!("## Recap\nBranch: main\nDid: x\nCommit/push: no\n\n## What Next\n{FENCE}"),
            TimelinePhase::Complete,
        )
        .unwrap();
        assert_eq!(legacy.source, None);
        assert_eq!(legacy.items[0].data["recap"]["did"], "x");
        let five = run(
            &format!(
                "## Recap\n- Branch: main\n- Commit/push: no\n- Did: x\n- Not yet: nothing\n- Need from you: y\n\n## What Next\n{FENCE}"
            ),
            TimelinePhase::Complete,
        )
        .unwrap();
        assert_eq!(five.items[0].data["recap"]["need"], "y");
    }
}
