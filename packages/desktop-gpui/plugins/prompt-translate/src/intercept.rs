//! The `rewrite-english` interceptor (`index.client.tsx:42`) and the `prompt-note` transformer
//! (`index.client.tsx:25`), as pure functions.

use crate::settings::{Settings, rewrites_on_send_key};
use crate::vietnamese::has_vietnamese;
use bachuc_plugin_host::{
    InterceptOutcome, PluginTimelineItem, SendKey, SourcePlacement, SourcePosition, TimelinePhase,
    TimelineTransformInput, TimelineTransformResult,
};
use serde_json::{Value, json};

/// What a send needs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Plan {
    /// Send the draft as typed.
    Keep,
    /// Ask `translate.enhance` for the rewrite.
    Enhance,
}

/// A one-line English command such as `/compact` or `$caveman ultra` reaches the agent as
/// typed (`index.client.tsx:49`).
fn is_one_line_command(text: &str) -> bool {
    let text = text.trim();
    text.starts_with(['/', '$']) && !text.contains('\n')
}

pub(crate) fn plan(settings: &Settings, text: &str, send_key: Option<SendKey>) -> Plan {
    // The host setting decides; the pill only picks the Caveman mode.
    if !settings.enhance_shortcut || text.trim().is_empty() {
        return Plan::Keep;
    }
    // The key that does not rewrite in this mode sends the draft as typed.
    if !rewrites_on_send_key(send_key, settings.rewrite_mode) {
        return Plan::Keep;
    }
    if !has_vietnamese(text) && is_one_line_command(text) {
        return Plan::Keep;
    }
    Plan::Enhance
}

/// The outcome of an enhance call, and the error text to show. Electron keeps the draft and
/// shows the error as the composer's send error; here `Cancel` keeps the draft and a message
/// shows the text.
pub(crate) fn outcome(answer: anyhow::Result<String>) -> (InterceptOutcome, Option<String>) {
    match answer {
        Ok(prompt) => (InterceptOutcome::Replace(prompt), None),
        Err(error) => (InterceptOutcome::Cancel, Some(format!("{error:#}"))),
    }
}

pub(crate) const NOTE_KIND: &str = "prompt-note";
pub(crate) const NOTE_VERSION: u32 = 1;

/// The note item of a completed user message. It reads the item and `translate` only, so it
/// stays a pure function of its input (ADR-0039).
pub(crate) fn transform(
    input: &TimelineTransformInput<'_>,
    translate: bool,
) -> Option<TimelineTransformResult> {
    if input.phase != TimelinePhase::Complete || !translate {
        return None;
    }
    let text = input.item.get("text").and_then(Value::as_str)?;
    if text.trim().is_empty() {
        return None;
    }
    Some(TimelineTransformResult {
        items: vec![PluginTimelineItem {
            id: None,
            kind: NOTE_KIND.into(),
            version: NOTE_VERSION,
            data: json!({ "text": text }),
        }],
        source: Some(SourcePlacement {
            position: SourcePosition::First,
            text: None,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::{Plan, outcome, plan, transform};
    use crate::settings::{RewriteMode, Settings};
    use bachuc_plugin_host::{
        InterceptOutcome, SendKey, SourcePosition, TimelinePhase, TimelineTransformInput,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn transformer_cases() {
        let item = json!({ "type": "user_message", "text": "Giải thích" });
        let labels = BTreeMap::new();
        let complete = TimelineTransformInput {
            agent_id: "a1",
            labels: &labels,
            item: &item,
            phase: TimelinePhase::Complete,
        };
        let result = transform(&complete, true).expect("a complete item with text");
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].kind, "prompt-note");
        assert_eq!(result.items[0].version, 1);
        assert_eq!(result.items[0].id, None);
        assert_eq!(result.items[0].data, json!({ "text": "Giải thích" }));
        let source = result.source.expect("the source stays");
        assert_eq!(source.position, SourcePosition::First);
        assert_eq!(source.text, None);
        // Streaming, translate off, a blank text, and a missing text give nothing.
        let streaming = TimelineTransformInput {
            phase: TimelinePhase::Streaming,
            ..complete
        };
        assert!(transform(&streaming, true).is_none());
        assert!(transform(&complete, false).is_none());
        let blank = json!({ "type": "user_message", "text": "  \n " });
        let blank = TimelineTransformInput {
            item: &blank,
            ..complete
        };
        assert!(transform(&blank, true).is_none());
        let none = json!({ "type": "user_message" });
        let none = TimelineTransformInput {
            item: &none,
            ..complete
        };
        assert!(transform(&none, true).is_none());
    }

    #[test]
    fn interceptor_keeps_commands() {
        let settings = Settings::default();
        let enter = Some(SendKey::Enter);
        assert_eq!(plan(&settings, "/compact", enter), Plan::Keep);
        assert_eq!(plan(&settings, "$caveman ultra", enter), Plan::Keep);
        assert_eq!(plan(&settings, "  /compact  ", None), Plan::Keep);
        // A Vietnamese one-line command is rewritten; a command over two lines too.
        assert_eq!(plan(&settings, "/giải thích lỗi này", enter), Plan::Enhance);
        assert_eq!(plan(&settings, "/compact\nxin chào", enter), Plan::Enhance);
        assert_eq!(plan(&settings, "cái này là gì", enter), Plan::Enhance);
        assert_eq!(plan(&settings, "explain this", None), Plan::Enhance);
        // The setting is off, the draft is blank, or the key does not rewrite in this mode.
        let off = Settings {
            enhance_shortcut: false,
            ..Settings::default()
        };
        assert_eq!(plan(&off, "cái này là gì", enter), Plan::Keep);
        assert_eq!(plan(&settings, "  \n", enter), Plan::Keep);
        assert_eq!(
            plan(&settings, "cái này là gì", Some(SendKey::ModEnter)),
            Plan::Keep
        );
        let cmd = Settings {
            rewrite_mode: RewriteMode::CmdEnter,
            ..Settings::default()
        };
        assert_eq!(plan(&cmd, "cái này là gì", enter), Plan::Keep);
        assert_eq!(
            plan(&cmd, "cái này là gì", Some(SendKey::ModEnter)),
            Plan::Enhance
        );
    }

    #[test]
    fn interceptor_cancels_on_error() {
        assert_eq!(
            outcome(Ok("Explain this".into())),
            (InterceptOutcome::Replace("Explain this".into()), None)
        );
        let (result, message) = outcome(Err(anyhow::anyhow!("provider timed out")));
        assert_eq!(result, InterceptOutcome::Cancel);
        assert_eq!(message.as_deref(), Some("provider timed out"));
    }
}
