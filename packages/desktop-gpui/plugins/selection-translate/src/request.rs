//! The `selection.translate` RPC (`shared/contracts.ts`): the length rules, the input, and the
//! answer decoder.

use anyhow::Context as _;
use serde_json::{Value, json};

pub(crate) const TRANSLATE: &str = "selection.translate";

/// The server limit, in UTF-16 units of the trimmed text. Copied from the Cite crate, whose
/// `utf16_len` is private and which a plugin crate must not depend on.
const MAX_SELECTION_LENGTH: usize = 4000;

/// `{text}` for the RPC. An empty or over-long selection makes no request.
pub(crate) fn request_input(text: &str) -> anyhow::Result<Value> {
    let text = text.trim();
    if text.is_empty() {
        anyhow::bail!("Không có chữ để dịch");
    }
    if text.encode_utf16().count() > MAX_SELECTION_LENGTH {
        anyhow::bail!("Đoạn chọn dài hơn {MAX_SELECTION_LENGTH} ký tự");
    }
    Ok(json!({ "text": text }))
}

/// `{translation: string}`.
pub(crate) fn parse_translation(answer: &Value) -> anyhow::Result<String> {
    answer
        .get("translation")
        .and_then(Value::as_str)
        .map(str::to_string)
        .context("the answer has no `translation` text")
}

#[cfg(test)]
mod tests {
    use super::{parse_translation, request_input};
    use serde_json::json;

    #[test]
    fn the_input_carries_the_text() {
        assert_eq!(request_input("Hello").unwrap(), json!({ "text": "Hello" }));
    }

    #[test]
    fn empty_and_over_long_text_make_no_request() {
        assert_eq!(
            request_input("  ").unwrap_err().to_string(),
            "Không có chữ để dịch"
        );
        let long = "a".repeat(4001);
        assert_eq!(
            request_input(&long).unwrap_err().to_string(),
            "Đoạn chọn dài hơn 4000 ký tự"
        );
        assert!(request_input(&"a".repeat(4000)).is_ok());
        // An emoji is 2 UTF-16 units, so 2000 of them fill the limit and one more passes it.
        assert!(request_input(&"😀".repeat(2000)).is_ok());
        assert!(request_input(&"😀".repeat(2001)).is_err());
        assert_eq!(
            request_input("  Hello \n").unwrap(),
            json!({ "text": "Hello" })
        );
    }

    #[test]
    fn the_answer_is_the_translation_text() {
        assert_eq!(
            parse_translation(&json!({ "translation": "Xin chào" })).unwrap(),
            "Xin chào"
        );
        assert!(parse_translation(&json!({})).is_err());
    }
}
