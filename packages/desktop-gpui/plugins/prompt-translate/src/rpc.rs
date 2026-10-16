//! The server RPCs of the plugin (`shared/contracts.ts`): names, inputs, and answer decoders.

use crate::notes::Request;
use crate::settings::Mode;
use anyhow::Context as _;
use serde_json::{Value, json};

pub(crate) const TRANSLATE: &str = "translate.translate";
pub(crate) const ENHANCE: &str = "translate.enhance";
pub(crate) const ORIGINAL: &str = "translate.original";
pub(crate) const MODE_READ: &str = "translate.mode.read";
pub(crate) const MODE_WRITE: &str = "translate.mode.write";

/// The method, input, and answer field of a note lookup.
pub(crate) fn lookup_call(request: &Request) -> (&'static str, Value, &'static str) {
    match request {
        Request::Translate { text, cache_only } => (
            TRANSLATE,
            json!({ "text": text, "cacheOnly": cache_only }),
            "translation",
        ),
        Request::Original { text } => (ORIGINAL, json!({ "text": text }), "original"),
    }
}

/// `{translation: string | null}` and `{original: string | null}`.
pub(crate) fn nullable_text(answer: &Value, field: &str) -> anyhow::Result<Option<String>> {
    match answer.get(field) {
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(Value::Null) => Ok(None),
        _ => anyhow::bail!("the answer has no `{field}` text or null"),
    }
}

pub(crate) fn enhance_input(text: &str) -> Value {
    json!({ "text": text, "deferCaveman": true })
}

/// `{prompt: string}`.
pub(crate) fn parse_prompt(answer: &Value) -> anyhow::Result<String> {
    answer
        .get("prompt")
        .and_then(Value::as_str)
        .map(str::to_string)
        .context("the answer has no `prompt` text")
}

pub(crate) fn mode_read_input(agent_id: &str) -> Value {
    json!({ "agentId": agent_id })
}

pub(crate) fn mode_write_input(agent_id: &str, mode: Mode) -> Value {
    json!({ "agentId": agent_id, "mode": mode.value() })
}

/// `{mode}`.
pub(crate) fn parse_mode(answer: &Value) -> anyhow::Result<Mode> {
    let value = answer
        .get("mode")
        .and_then(Value::as_str)
        .context("the answer has no `mode` text")?;
    Mode::parse(value).with_context(|| format!("unknown Caveman mode `{value}`"))
}

#[cfg(test)]
mod tests {
    use super::{
        enhance_input, lookup_call, mode_write_input, nullable_text, parse_mode, parse_prompt,
    };
    use crate::notes::request_for;
    use crate::settings::Mode;
    use serde_json::json;

    #[test]
    fn the_rpc_inputs_and_answers_follow_contracts_ts() {
        let (method, input, field) = lookup_call(&request_for("Giải thích", true));
        assert_eq!(
            (method, input, field),
            (
                "translate.translate",
                json!({ "text": "Giải thích", "cacheOnly": true }),
                "translation"
            )
        );
        let (method, input, field) = lookup_call(&request_for("Explain", false));
        assert_eq!(
            (method, input, field),
            (
                "translate.original",
                json!({ "text": "Explain" }),
                "original"
            )
        );
        assert_eq!(
            enhance_input("xin chào"),
            json!({ "text": "xin chào", "deferCaveman": true })
        );
        assert_eq!(
            mode_write_input("a1", Mode::Ultra),
            json!({ "agentId": "a1", "mode": "ultra" })
        );
        assert_eq!(
            nullable_text(&json!({ "translation": "Hi" }), "translation").unwrap(),
            Some("Hi".into())
        );
        assert_eq!(
            nullable_text(&json!({ "original": null }), "original").unwrap(),
            None
        );
        assert!(nullable_text(&json!({}), "original").is_err());
        assert_eq!(
            parse_prompt(&json!({ "prompt": "Explain" })).unwrap(),
            "Explain"
        );
        assert!(parse_prompt(&json!({})).is_err());
        assert_eq!(
            parse_mode(&json!({ "mode": "wenyan-full" })).unwrap(),
            Mode::WenyanFull
        );
        assert!(parse_mode(&json!({ "mode": "nope" })).is_err());
    }
}
