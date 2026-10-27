//! The composer contribution kinds: pills, send interceptors, stop buttons, and attachment
//! sources, with the draft values and attachment chips they read and write.

use crate::context::PluginContext;
use crate::{Handler, PopoverClose};
use bachuc_icons::IconName;
use gpui_kit::{
    AnyElement, AnyView, App, AppContext as _, Entity, Hsla, Pixels, SharedString, Task, Window,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::rc::Rc;

/// A composer pill (`PluginComposerPillContribution`, `buttons.ts:111`). Leave `workspace_id`
/// and `agent_id` `None` to show it on every matching agent composer.
#[derive(Clone)]
pub struct ComposerPill {
    pub id: &'static str,
    pub placement: PillPlacement,
    pub show_on_draft: ShowOnDraft,
    pub workspace_id: Option<String>,
    pub agent_id: Option<String>,
    /// `None` hides the pill.
    pub state: Rc<dyn Fn(&ComposerTarget, &App) -> Option<PillState>>,
}

/// Where a pill sits. Compact layouts keep every pill in the track; the composer owns that
/// rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PillPlacement {
    #[default]
    Track,
    Toolbar,
    Corner,
    /// An icon in the joined button group of the workspace header, for the focused agent tab.
    /// The label is not drawn; the title is the tooltip.
    Header,
}

/// Whether a pill shows on the New workspace draft composer. `Only` skips agent composers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShowOnDraft {
    #[default]
    No,
    Yes,
    Only,
}

#[derive(Clone)]
pub struct PillState {
    pub title: SharedString,
    pub label: Option<SharedString>,
    pub icon: Option<IconName>,
    /// The icon, label, and border color; `None` uses the host's muted tone.
    pub color: Option<Hsla>,
    pub disabled: bool,
    pub behavior: PillBehavior,
}

/// What a pill press does (`behaviors`, `buttons.ts:30`).
#[derive(Clone)]
pub enum PillBehavior {
    Action {
        on_press: Handler<ComposerTarget>,
        /// Opens this New workspace panel on a draft composer instead.
        new_workspace_panel: Option<&'static str>,
    },
    Select {
        options: Vec<SelectOption>,
        value: Option<SharedString>,
        on_select: Rc<dyn Fn(&ComposerTarget, &str, &mut App)>,
    },
    Popover {
        flush: bool,
        content: Rc<dyn Fn(ComposerTarget, PopoverClose, &mut Window, &mut App) -> AnyView>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    pub id: SharedString,
    pub label: SharedString,
    pub description: Option<SharedString>,
}

/// The composer a pill or interceptor acts on. A draft composer has `draft` and no agent.
#[derive(Clone)]
pub struct ComposerTarget {
    pub workspace_id: Option<String>,
    pub agent_id: Option<String>,
    pub draft: Option<Entity<ComposerDraft>>,
}

/// The plugin values of one New workspace draft (`PluginComposerDraftState`, `draft.ts`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComposerDraft {
    pub cwd: Option<String>,
    pub values: BTreeMap<String, Value>,
}

impl ComposerDraft {
    pub fn get(&self, plugin_id: &str) -> Option<&Value> {
        self.values.get(plugin_id)
    }

    pub fn set(&mut self, plugin_id: &str, value: Value) {
        self.values.insert(plugin_id.to_string(), value);
    }

    /// The env of the first agent: one `PASEO_PLUGIN_COMPOSER_<plugin_id>` key per plugin,
    /// with the JSON of its value, as `draft.ts:24`.
    pub fn environment(&self) -> BTreeMap<String, String> {
        self.values
            .iter()
            .map(|(id, value)| (format!("PASEO_PLUGIN_COMPOSER_{id}"), value.to_string()))
            .collect()
    }
}

/// Whether `pill` shows on a composer. A draft composer shows only unscoped pills that opt in
/// with `Yes` or `Only`; an agent composer hides `Only` pills and pills scoped to another
/// workspace or agent (`buttons.ts:111`, `buttons/model.ts`).
pub fn pill_applies(
    pill: &ComposerPill,
    workspace_id: Option<&str>,
    agent_id: Option<&str>,
    is_draft: bool,
) -> bool {
    if is_draft {
        return pill.workspace_id.is_none()
            && pill.agent_id.is_none()
            && pill.show_on_draft != ShowOnDraft::No;
    }
    let scoped_to = |scope: &Option<String>, current: Option<&str>| {
        scope.as_deref().is_none_or(|scope| Some(scope) == current)
    };
    pill.show_on_draft != ShowOnDraft::Only
        && scoped_to(&pill.workspace_id, workspace_id)
        && scoped_to(&pill.agent_id, agent_id)
}

/// Runs before the composer sends or queues `text`.
#[derive(Clone)]
pub struct ComposerInterceptor {
    pub id: &'static str,
    pub intercept: Rc<dyn Fn(InterceptInput, &mut App) -> Task<InterceptOutcome>>,
}

#[derive(Clone)]
pub struct InterceptInput {
    pub target: ComposerTarget,
    pub text: String,
    pub action: SendAction,
    pub send_key: Option<SendKey>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendAction {
    Send,
    Queue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendKey {
    Enter,
    ModEnter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterceptOutcome {
    Keep,
    Replace(String),
    Cancel,
}

/// Draws the composer's stop button while an agent runs. The first one registered wins.
#[derive(Clone)]
pub struct StopButton {
    pub id: &'static str,
    pub render: Rc<dyn Fn(&StopButtonProps, &mut Window, &mut App) -> AnyElement>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StopButtonProps {
    pub agent_id: String,
    pub size: Pixels,
    pub icon_size: Pixels,
    pub cancelling: bool,
}

/// An attachment menu entry (`PluginAttachmentSourceContribution`). With `picker`, the
/// composer opens a search; `on_open` runs when the user presses a chip of this source.
#[derive(Clone)]
pub struct AttachmentSource {
    pub id: &'static str,
    pub title: SharedString,
    pub icon: IconName,
    pub picker: Option<AttachmentPicker>,
    pub on_open: Option<Handler<AttachmentItem>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentPicker {
    pub title: SharedString,
    pub search_placeholder: SharedString,
    /// The plugin RPC method that answers `{query}` with `{items}`.
    pub search_method: &'static str,
}

/// `PluginAttachmentItemSchema` (`attachments.ts:10`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentItem {
    pub id: String,
    pub identifier: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub url: String,
    pub text: String,
    pub resource_type: String,
}

/// One plugin chip in an agent composer.
#[derive(Clone, Debug, PartialEq)]
pub struct ComposerAttachment {
    pub agent_id: String,
    pub plugin_id: String,
    pub source_id: String,
    pub source_title: String,
    pub icon: IconName,
    pub item: AttachmentItem,
    pub commentable: bool,
}

impl ComposerAttachment {
    /// The `AgentAttachment` JSON the agent gets, as `pluginResourceAttachmentToAgentAttachment`
    /// (`attachments/model.ts:76`). The comment is added only when the chip is commentable and
    /// the trimmed comment is not empty.
    pub fn to_agent_attachment(&self, comment: Option<&str>) -> Value {
        let comment = comment
            .filter(|_| self.commentable)
            .map(str::trim)
            .filter(|comment| !comment.is_empty());
        let item = &self.item;
        let text = match comment {
            Some(comment) => format!("{}\n\nComment: {comment}", item.text),
            None => item.text.clone(),
        };
        json!({
            "type": "text",
            "mimeType": "text/plain",
            "title": format!("{} {}", item.identifier, item.title),
            "text": text,
            "externalResource": {
                "provider": self.plugin_id,
                "providerLabel": self.source_title,
                "resourceType": item.resource_type,
                "id": item.id,
                "identifier": item.identifier,
                "title": item.title,
                "url": item.url,
            },
        })
    }
}

#[derive(Deserialize)]
struct AttachmentSearchPayload {
    items: Vec<AttachmentItem>,
}

/// Decodes a picker search answer (`PluginAttachmentSearchPayloadSchema`).
pub fn decode_attachment_search(output: Value) -> anyhow::Result<Vec<AttachmentItem>> {
    let payload: AttachmentSearchPayload = serde_json::from_value(output)?;
    Ok(payload.items)
}

/// Calls the picker's search RPC with `{query}` and decodes its `{items}`.
pub fn search_attachments(
    ctx: &PluginContext,
    picker: &AttachmentPicker,
    query: &str,
    cx: &mut App,
) -> Task<anyhow::Result<Vec<AttachmentItem>>> {
    let answer = ctx.rpc(picker.search_method, json!({ "query": query }), cx);
    cx.background_spawn(async move { decode_attachment_search(answer.await?) })
}
