//! `bachuc-plugin-assistant-cite`: the native client of the assistant-cite plugin
//! (`hiep-plugins/plugins/assistant-cite/index.client.tsx`, ADR-0045). It adds the `Cite`
//! selection action, which turns selected assistant text into a `Quote` chip in the composer, and
//! the chip-only `quote` source, whose chip scrolls back to the passage.

mod format;
mod item;

use crate::format::{MAX_QUOTE_LENGTH, QuoteFailure};
use crate::item::{QUOTE_SOURCE_ID, QuoteSource, build_quote_item, parse_quote_item};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    AssistantSelection, AttachmentItem, AttachmentSource, ComposerAttachment, Contributions,
    NativePlugin, PluginContext, SelectionAction, TimelinePassage,
};
use gpui_kit::App;
use std::rc::Rc;

const PLUGIN_ID: &str = "assistant-cite";
const SOURCE_TITLE: &str = "Quote";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(AssistantCite)
}

struct AssistantCite;

impl NativePlugin for AssistantCite {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, _cx: &mut App) {
        // A chip-only source: it has no picker and opens quote chips.
        let open_ctx = ctx.clone();
        contributions.add_attachment_source(AttachmentSource {
            id: QUOTE_SOURCE_ID,
            title: SOURCE_TITLE.into(),
            icon: IconName::Quote,
            picker: None,
            on_open: Some(Rc::new(move |item, _window, cx| {
                open_quote(&open_ctx, &item, cx)
            })),
        });
        contributions.add_selection_action(SelectionAction {
            id: "cite",
            title: "Cite".into(),
            on_select: Rc::new(move |selection, _window, cx| cite(&ctx, selection, cx)),
        });
    }
}

/// The `server_id` of the active host; `None` before the client connects.
fn active_server_id(ctx: &PluginContext, cx: &App) -> Option<String> {
    let client = ctx.client(cx)?;
    client
        .read(cx)
        .server_info()
        .map(|info| info.server_id.clone())
}

/// What a chip press scrolls to: its passage, only on the host the chip came from. A chip of
/// another host, or one pressed before connect, does nothing.
fn reveal_target(item: &AttachmentItem, active_server: Option<&str>) -> Option<TimelinePassage> {
    let passage = parse_quote_item(item)?;
    (active_server == Some(passage.server_id.as_str())).then(|| TimelinePassage {
        agent_id: passage.agent_id,
        message_id: passage.message_id,
        text: passage.text,
    })
}

fn open_quote(ctx: &PluginContext, item: &AttachmentItem, cx: &mut App) {
    let server = active_server_id(ctx, cx);
    if let Some(passage) = reveal_target(item, server.as_deref()) {
        ctx.reveal_timeline_passage(passage, cx);
    }
}

/// Why a selection adds no chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CiteFailure {
    /// The client has not connected, so the chip has no server to name.
    NoServer,
    Quote(QuoteFailure),
}

/// The commentable `Quote` chip for a selection of assistant text.
fn cite_attachment(
    server_id: Option<&str>,
    selection: &AssistantSelection,
) -> Result<ComposerAttachment, CiteFailure> {
    let server_id = server_id.ok_or(CiteFailure::NoServer)?;
    let source = QuoteSource {
        server_id,
        agent_id: &selection.agent_id,
        message_id: selection.message_id.as_deref(),
    };
    let item = build_quote_item(source, &selection.text).map_err(CiteFailure::Quote)?;
    Ok(ComposerAttachment {
        agent_id: selection.agent_id.clone(),
        plugin_id: PLUGIN_ID.into(),
        source_id: QUOTE_SOURCE_ID.into(),
        source_title: SOURCE_TITLE.into(),
        icon: IconName::Quote,
        item,
        commentable: true,
    })
}

fn cite(ctx: &PluginContext, selection: AssistantSelection, cx: &mut App) {
    let server = active_server_id(ctx, cx);
    match cite_attachment(server.as_deref(), &selection) {
        Ok(attachment) => ctx.add_composer_attachment(attachment, cx),
        // Plugins have no toast for this in Electron either (Q-011), so an over-long quote only
        // logs.
        Err(CiteFailure::Quote(QuoteFailure::TooLong)) => {
            log::warn!("[assistant-cite] Quotes are limited to {MAX_QUOTE_LENGTH} characters");
        }
        Err(CiteFailure::Quote(QuoteFailure::Empty)) => {}
        Err(CiteFailure::NoServer) => log::warn!("[assistant-cite] No connected host to cite from"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_plugin_host::ContributionIndex;

    fn selection(message_id: Option<&str>, text: &str) -> AssistantSelection {
        AssistantSelection {
            workspace_id: Some("w1".into()),
            agent_id: "agent-1".into(),
            message_id: message_id.map(str::to_string),
            text: text.into(),
        }
    }

    fn chip(server: &str) -> AttachmentItem {
        cite_attachment(Some(server), &selection(Some("msg-1"), "Same words"))
            .unwrap()
            .item
    }

    #[test]
    fn cite_builds_a_commentable_quote_chip() {
        let attachment =
            cite_attachment(Some("host-1"), &selection(Some("msg-1"), "Same words")).unwrap();
        assert_eq!(attachment.agent_id, "agent-1");
        assert_eq!(attachment.plugin_id, "assistant-cite");
        assert_eq!(attachment.source_id, "quote");
        assert_eq!(attachment.source_title, "Quote");
        assert_eq!(attachment.icon, IconName::Quote);
        assert!(attachment.commentable);
        assert_eq!(attachment.item.id, "msg-1:6n05hy");
    }

    #[test]
    fn cite_adds_no_chip_without_a_server_or_for_a_bad_quote() {
        let ok = selection(Some("msg-1"), "x");
        assert_eq!(
            cite_attachment(None, &ok).unwrap_err(),
            CiteFailure::NoServer
        );
        assert_eq!(
            cite_attachment(Some("host-1"), &selection(None, "  ")).unwrap_err(),
            CiteFailure::Quote(QuoteFailure::Empty)
        );
        assert_eq!(
            cite_attachment(Some("host-1"), &selection(None, &"x".repeat(4001))).unwrap_err(),
            CiteFailure::Quote(QuoteFailure::TooLong)
        );
    }

    #[test]
    fn this_hosts_chip_reveals_its_passage() {
        assert_eq!(
            reveal_target(&chip("host-1"), Some("host-1")),
            Some(TimelinePassage {
                agent_id: "agent-1".into(),
                message_id: "msg-1".into(),
                text: Some("Same words".into()),
            })
        );
    }

    #[test]
    fn other_host_chip_reveals_nothing() {
        assert_eq!(reveal_target(&chip("host-2"), Some("host-1")), None);
        // Before connect there is no active server id.
        assert_eq!(reveal_target(&chip("host-1"), None), None);
    }

    #[test]
    fn a_chip_without_a_message_reveals_nothing() {
        let chip = cite_attachment(Some("host-1"), &selection(None, "Same words"))
            .unwrap()
            .item;
        assert_eq!(reveal_target(&chip, Some("host-1")), None);
    }

    #[test]
    fn the_source_and_action_ids() {
        let mut contributions = Contributions::default();
        contributions.add_attachment_source(AttachmentSource {
            id: QUOTE_SOURCE_ID,
            title: SOURCE_TITLE.into(),
            icon: IconName::Quote,
            picker: None,
            on_open: None,
        });
        let index = ContributionIndex::build([(PLUGIN_ID, &contributions)]);
        let [source] = index.attachment_sources() else {
            panic!("expected one source");
        };
        assert_eq!(source.value.id, "quote");
        assert!(source.value.picker.is_none());
    }
}
