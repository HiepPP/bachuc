use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

const MODEL: &str = "pplx-decider-flash-v1.2";

#[derive(Debug, PartialEq)]
pub enum Reply {
    Noul {
        p_true: f64,
    },
    Choice {
        choice: String,
        probabilities: BTreeMap<String, f64>,
    },
}

#[derive(Debug, PartialEq)]
pub struct Answer {
    pub input_tokens: Option<u64>,
    pub value: Reply,
}

/// Sends one question. A response that lacks an expected field is an `Err` with the parse error.
pub fn ask(endpoint: &str, state: &Value, qid: &str, question: &Value) -> Result<Answer, String> {
    let body = json!({ "model": MODEL, "state": state, "questions": { qid: question } });
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into();
    let mut response = agent
        .post(endpoint)
        .header("content-type", "application/json")
        .send(body.to_string())
        .map_err(|error| error.to_string())?;
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    parse_response(&text, qid)
}

pub fn parse_response(text: &str, qid: &str) -> Result<Answer, String> {
    let root: Value = serde_json::from_str(text).map_err(|error| format!("bad json: {error}"))?;
    let input_tokens = root.pointer("/usage/input_tokens").and_then(Value::as_u64);
    let answer = root
        .pointer(&format!("/answers/{qid}"))
        .ok_or_else(|| format!("missing answers.{qid}"))?;
    let kind = answer
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let value = match kind {
        "noul" => Reply::Noul {
            p_true: answer
                .get("noul")
                .and_then(Value::as_f64)
                .ok_or("missing field noul")?,
        },
        "choice" => {
            let choice = answer
                .get("choice")
                .and_then(Value::as_str)
                .ok_or("missing field choice")?
                .to_string();
            let probabilities = answer
                .get("probabilities")
                .and_then(Value::as_object)
                .ok_or("missing field probabilities")?
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), value.as_f64()?)))
                .collect();
            Reply::Choice {
                choice,
                probabilities,
            }
        }
        other => return Err(format!("unexpected answer type {other:?}")),
    };
    Ok(Answer {
        input_tokens,
        value,
    })
}

/// Length of the log in bytes, or `None` when the file is missing.
pub fn log_len(log: &Path) -> Option<u64> {
    fs::metadata(log).ok().map(|meta| meta.len())
}

/// Reads the bytes appended since `offset` and returns `ms=` of the last `decided` line.
pub fn server_ms_since(log: &Path, offset: Option<u64>) -> Option<u64> {
    let offset = offset?;
    for attempt in 0..4 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }
        let mut file = fs::File::open(log).ok()?;
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut tail = String::new();
        file.read_to_string(&mut tail).ok()?;
        if let Some(ms) = last_decided_ms(&tail) {
            return Some(ms);
        }
    }
    None
}

pub fn last_decided_ms(tail: &str) -> Option<u64> {
    let line = tail.lines().rev().find(|line| line.contains("decided"))?;
    let rest = &line[line.rfind(" ms=")? + 4..];
    rest.split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_noul_and_choice_answers() {
        let noul =
            r#"{"answers":{"readOnly":{"type":"noul","noul":0.9}},"usage":{"input_tokens":356}}"#;
        assert_eq!(
            parse_response(noul, "readOnly"),
            Ok(Answer {
                input_tokens: Some(356),
                value: Reply::Noul { p_true: 0.9 }
            })
        );
        let choice = r#"{"answers":{"action":{"type":"choice","choice":"allow","confidence":0.9,"probabilities":{"allow":0.99,"deny":0.0,"escalate":0.01}}}}"#;
        let answer = parse_response(choice, "action").unwrap();
        assert_eq!(answer.input_tokens, None);
        assert!(matches!(answer.value, Reply::Choice { ref choice, .. } if choice == "allow"));
    }

    #[test]
    fn missing_field_is_an_error() {
        let text = r#"{"answers":{"readOnly":{"type":"noul"}}}"#;
        assert_eq!(
            parse_response(text, "readOnly"),
            Err("missing field noul".to_string())
        );
    }

    #[test]
    fn reads_ms_of_the_last_decided_line() {
        let tail = "x INFO decided questions=1 passes=1 input_tokens=87 ms=3102\n\
                    x INFO decided questions=1 passes=1 input_tokens=94 ms=2721\nother line\n";
        assert_eq!(last_decided_ms(tail), Some(2721));
        assert_eq!(last_decided_ms("nothing here\n"), None);
    }
}
