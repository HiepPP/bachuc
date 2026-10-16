//! The host rules that need no `App`: gating, the index, composer pills and drafts, and
//! attachment chips.

use bachuc_client::{PluginListItem, PluginStatus};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    AttachmentItem, CommandCenterItem, CommandContext, ComposerAttachment, ComposerDraft,
    ComposerPill, ContributionIndex, Contributions, NativePlugin, PillPlacement, PluginContext,
    ShowOnDraft, Surface, TimelineRenderer, TimelineTransformResult, TimelineTransformer,
    ViewFactory, active_plugin_ids, decode_attachment_search, pill_applies,
};
use gpui_kit::{AnyView, App, Window};
use serde_json::json;
use std::rc::Rc;

fn view<I: 'static>() -> ViewFactory<I> {
    Rc::new(|_: I, _: &mut Window, _: &mut App| -> AnyView { unreachable!("no window in tests") })
}

fn listed(id: &str, enabled: bool, status: PluginStatus) -> PluginListItem {
    PluginListItem {
        id: id.into(),
        description: None,
        enabled,
        status,
        error: None,
    }
}

struct Fake(&'static str);

impl NativePlugin for Fake {
    fn id(&self) -> &'static str {
        self.0
    }

    fn contribute(&self, _: PluginContext, contributions: &mut Contributions, _: &mut App) {
        contributions.add_surface(surface(self.0));
    }
}

#[test]
fn native_plugin_is_object_safe() {
    let plugins: Vec<Box<dyn NativePlugin>> = vec![Box::new(Fake("a")), Box::new(Fake("b"))];
    let ids: Vec<_> = plugins.iter().map(|plugin| plugin.id()).collect();
    assert_eq!(ids, ["a", "b"]);
}

#[test]
fn only_enabled_running_listed_and_registered_plugins_are_active() {
    let listed = [
        listed("running", true, PluginStatus::Running),
        listed("off", false, PluginStatus::Running),
        listed("disabled", true, PluginStatus::Disabled),
        listed("failed", true, PluginStatus::Failed),
        listed("unknown", true, PluginStatus::Unknown),
        listed("not-registered", true, PluginStatus::Running),
    ];
    let registered = [
        "unknown",
        "failed",
        "running",
        "disabled",
        "off",
        "not-listed",
    ];
    assert_eq!(active_plugin_ids(&listed, &registered), ["running"]);
    assert!(active_plugin_ids(&[], &registered).is_empty());
}

#[test]
fn active_ids_follow_registration_order() {
    let listed = [
        listed("b", true, PluginStatus::Running),
        listed("a", true, PluginStatus::Running),
    ];
    assert_eq!(active_plugin_ids(&listed, &["a", "b"]), ["a", "b"]);
}

fn pill(id: &'static str) -> ComposerPill {
    ComposerPill {
        id,
        placement: PillPlacement::Track,
        show_on_draft: ShowOnDraft::No,
        workspace_id: None,
        agent_id: None,
        state: Rc::new(|_, _| None),
    }
}

fn command(id: &'static str) -> CommandCenterItem {
    CommandCenterItem {
        id,
        title: id.into(),
        icon: IconName::Blocks,
        keywords: Vec::new(),
        context: CommandContext::Global,
        visible: None,
        on_select: Rc::new(|_, _, _| {}),
    }
}

fn surface(id: &'static str) -> Surface {
    Surface {
        id,
        title: None,
        icon: None,
        view: view(),
    }
}

fn transformer(id: &'static str, item_type: &'static str) -> TimelineTransformer {
    TimelineTransformer {
        id,
        item_type,
        transform: Rc::new(|_| None::<TimelineTransformResult>),
    }
}

fn renderer(kind: &'static str, version: u32) -> TimelineRenderer {
    TimelineRenderer {
        kind,
        version,
        view: view(),
    }
}

fn two_plugins() -> (Contributions, Contributions) {
    let mut first = Contributions::default();
    first.add_composer_pill(pill("first-1"));
    first.add_composer_pill(pill("first-2"));
    first.add_command_item(command("first-command"));
    first.add_surface(surface("board"));
    first.add_timeline_transformer(transformer("first-tool", "tool_call"));
    first.add_timeline_transformer(transformer("first-message", "assistant_message"));
    first.add_timeline_renderer(renderer("job", 1));
    let mut second = Contributions::default();
    second.add_composer_pill(pill("second-1"));
    second.add_command_item(command("second-command"));
    second.add_surface(surface("board"));
    second.add_timeline_transformer(transformer("second-tool", "tool_call"));
    second.add_timeline_renderer(renderer("job", 1));
    second.add_timeline_renderer(renderer("job", 3));
    (first, second)
}

#[test]
fn the_index_keeps_plugin_order_then_call_order() {
    let (first, second) = two_plugins();
    let index = ContributionIndex::build([("first", &first), ("second", &second)]);
    let pills: Vec<_> = index
        .composer_pills()
        .iter()
        .map(|pill| (pill.plugin_id, pill.value.id))
        .collect();
    assert_eq!(
        pills,
        [
            ("first", "first-1"),
            ("first", "first-2"),
            ("second", "second-1")
        ]
    );
    let commands: Vec<_> = index
        .command_items()
        .iter()
        .map(|item| item.value.id)
        .collect();
    assert_eq!(commands, ["first-command", "second-command"]);
    assert!(index.sidebar_items().is_empty());
}

#[test]
fn transformers_are_looked_up_by_item_type() {
    let (first, second) = two_plugins();
    let index = ContributionIndex::build([("first", &first), ("second", &second)]);
    let tool: Vec<_> = index
        .transformers_for("tool_call")
        .iter()
        .map(|transformer| transformer.value.id)
        .collect();
    assert_eq!(tool, ["first-tool", "second-tool"]);
    let message = index.transformers_for("assistant_message");
    assert_eq!(message.len(), 1);
    assert_eq!(message[0].value.id, "first-message");
    assert!(index.transformers_for("reasoning").is_empty());
}

#[test]
fn renderers_match_kind_and_version_and_the_first_wins() {
    let (first, second) = two_plugins();
    let index = ContributionIndex::build([("first", &first), ("second", &second)]);
    assert!(index.renderer_for("job", 2).is_none());
    assert!(index.renderer_for("other", 1).is_none());
    assert_eq!(index.renderer_for("job", 1).unwrap().plugin_id, "first");
    assert_eq!(index.renderer_for("job", 3).unwrap().plugin_id, "second");
}

#[test]
fn surfaces_are_found_by_plugin_and_id() {
    let (first, second) = two_plugins();
    let index = ContributionIndex::build([("first", &first), ("second", &second)]);
    assert_eq!(
        index.surface("second", "board").unwrap().plugin_id,
        "second"
    );
    assert_eq!(index.surface("first", "board").unwrap().plugin_id, "first");
    assert!(index.surface("first", "missing").is_none());
    assert!(index.surface("third", "board").is_none());
}

#[test]
fn draft_composers_show_only_unscoped_opted_in_pills() {
    let no = pill("no");
    let yes = ComposerPill {
        show_on_draft: ShowOnDraft::Yes,
        ..pill("yes")
    };
    let only = ComposerPill {
        show_on_draft: ShowOnDraft::Only,
        ..pill("only")
    };
    assert!(!pill_applies(&no, None, None, true));
    assert!(pill_applies(&yes, None, None, true));
    assert!(pill_applies(&only, None, None, true));
    let scoped = ComposerPill {
        workspace_id: Some("w1".into()),
        ..yes.clone()
    };
    assert!(!pill_applies(&scoped, Some("w1"), None, true));
    let agent_scoped = ComposerPill {
        agent_id: Some("a1".into()),
        ..yes.clone()
    };
    assert!(!pill_applies(&agent_scoped, None, None, true));
}

#[test]
fn agent_composers_hide_only_pills_and_pills_scoped_elsewhere() {
    let only = ComposerPill {
        show_on_draft: ShowOnDraft::Only,
        ..pill("only")
    };
    assert!(!pill_applies(&only, Some("w1"), Some("a1"), false));
    assert!(pill_applies(&pill("any"), Some("w1"), Some("a1"), false));
    let yes = ComposerPill {
        show_on_draft: ShowOnDraft::Yes,
        ..pill("yes")
    };
    assert!(pill_applies(&yes, Some("w1"), Some("a1"), false));
    let other_agent = ComposerPill {
        agent_id: Some("a2".into()),
        ..pill("other-agent")
    };
    assert!(!pill_applies(&other_agent, Some("w1"), Some("a1"), false));
    let this_agent = ComposerPill {
        agent_id: Some("a1".into()),
        ..pill("this-agent")
    };
    assert!(pill_applies(&this_agent, Some("w1"), Some("a1"), false));
    let other_workspace = ComposerPill {
        workspace_id: Some("w2".into()),
        ..pill("other-workspace")
    };
    assert!(!pill_applies(
        &other_workspace,
        Some("w1"),
        Some("a1"),
        false
    ));
}

#[test]
fn draft_environment_has_one_json_key_per_plugin() {
    let mut draft = ComposerDraft::default();
    draft.set("skill-pins", json!({"skills": ["a", "b"]}));
    draft.set("prompt-translate", json!("ultra"));
    draft.set("flag", json!(true));
    assert_eq!(draft.get("flag"), Some(&json!(true)));
    assert_eq!(draft.get("missing"), None);
    let env = draft.environment();
    assert_eq!(env.len(), 3);
    assert_eq!(
        env["PASEO_PLUGIN_COMPOSER_skill-pins"],
        r#"{"skills":["a","b"]}"#
    );
    assert_eq!(env["PASEO_PLUGIN_COMPOSER_prompt-translate"], r#""ultra""#);
    assert_eq!(env["PASEO_PLUGIN_COMPOSER_flag"], "true");
}

fn item() -> AttachmentItem {
    AttachmentItem {
        id: "issue-1".into(),
        identifier: "#12".into(),
        title: "Fix the pill".into(),
        subtitle: None,
        url: "https://example.com/12".into(),
        text: "Body".into(),
        resource_type: "issue".into(),
    }
}

fn chip(commentable: bool) -> ComposerAttachment {
    ComposerAttachment {
        agent_id: "agent-1".into(),
        plugin_id: "linear".into(),
        source_id: "issues".into(),
        source_title: "Linear".into(),
        icon: IconName::Blocks,
        item: item(),
        commentable,
    }
}

#[test]
fn a_chip_becomes_a_text_attachment_with_its_resource() {
    let attachment = chip(true).to_agent_attachment(Some("  please check  "));
    assert_eq!(
        attachment,
        json!({
            "type": "text",
            "mimeType": "text/plain",
            "title": "#12 Fix the pill",
            "text": "Body\n\nComment: please check",
            "externalResource": {
                "provider": "linear",
                "providerLabel": "Linear",
                "resourceType": "issue",
                "id": "issue-1",
                "identifier": "#12",
                "title": "Fix the pill",
                "url": "https://example.com/12",
            },
        })
    );
}

#[test]
fn a_blank_comment_or_a_plain_chip_adds_no_comment() {
    assert_eq!(chip(true).to_agent_attachment(Some("   "))["text"], "Body");
    assert_eq!(chip(true).to_agent_attachment(None)["text"], "Body");
    assert_eq!(
        chip(false).to_agent_attachment(Some("please check"))["text"],
        "Body"
    );
}

#[test]
fn a_search_payload_decodes_into_items() {
    let items = decode_attachment_search(json!({
        "items": [
            {
                "id": "issue-1",
                "identifier": "#12",
                "title": "Fix the pill",
                "url": "https://example.com/12",
                "text": "Body",
                "resourceType": "issue",
            },
            {
                "id": "issue-2",
                "identifier": "#13",
                "title": "Second",
                "subtitle": "Open",
                "url": "https://example.com/13",
                "text": "",
                "resourceType": "issue",
            },
        ],
    }))
    .unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], item());
    assert_eq!(items[1].subtitle.as_deref(), Some("Open"));
    assert!(decode_attachment_search(json!({"items": [{"id": "x"}]})).is_err());
    assert!(decode_attachment_search(json!({})).is_err());
}
