mod client;
mod metrics;
mod report;

use client::{Reply, ask};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:8080/v1/decisions";
const DEFAULT_LOG: &str = "Library/Application Support/ai.perplexity.decider/decider.log";

#[derive(Serialize, Deserialize, Clone)]
pub struct ReadOnlyAnswer {
    pub p_true: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionAnswer {
    pub choice: String,
    pub probabilities: BTreeMap<String, f64>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Answers {
    #[serde(rename = "readOnly", default, skip_serializing_if = "Option::is_none")]
    pub read_only: Option<ReadOnlyAnswer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionAnswer>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ResultLine {
    pub backend: String,
    pub id: String,
    pub question_id: String,
    pub question_version: String,
    pub input_tokens: Option<u64>,
    pub client_ms: f64,
    pub server_ms: Option<u64>,
    pub answers: Answers,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct Event {
    pub id: String,
    pub state: Value,
    #[serde(default)]
    pub meta: Value,
}

struct Args {
    command: String,
    data: PathBuf,
    endpoint: String,
    questions: PathBuf,
    log: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("usage: decider-probe <run|report> --data <dir>")?;
    let home = std::env::var("HOME").unwrap_or_default();
    let mut parsed = Args {
        command,
        data: PathBuf::new(),
        endpoint: DEFAULT_ENDPOINT.to_string(),
        questions: Path::new(env!("CARGO_MANIFEST_DIR")).join("questions.json"),
        log: Path::new(&home).join(DEFAULT_LOG),
    };
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--data" => parsed.data = value.into(),
            "--endpoint" => parsed.endpoint = value,
            "--questions" => parsed.questions = value.into(),
            "--log" => parsed.log = value.into(),
            other => return Err(format!("unknown flag {other}")),
        }
    }
    if parsed.data.as_os_str().is_empty() {
        return Err("--data <dir> is required".to_string());
    }
    Ok(parsed)
}

pub fn read_jsonl<T: for<'de> Deserialize<'de>>(file: &Path) -> Result<Vec<T>, String> {
    let Ok(text) = fs::read_to_string(file) else {
        return Ok(Vec::new());
    };
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line)
                .map_err(|error| format!("{}:{}: {error}", file.display(), index + 1))
        })
        .collect()
}

fn run(args: &Args) -> Result<bool, String> {
    let spec: Value = serde_json::from_str(
        &fs::read_to_string(&args.questions)
            .map_err(|error| format!("{}: {error}", args.questions.display()))?,
    )
    .map_err(|error| format!("questions: {error}"))?;
    let version = spec["version"].as_str().unwrap_or_default().to_string();
    let questions = spec["questions"]
        .as_object()
        .ok_or("questions.json has no questions")?;
    let events: Vec<Event> = read_jsonl(&args.data.join("events.jsonl"))?;
    let results_path = args.data.join("results-decider.jsonl");
    let done: HashSet<(String, String)> = read_jsonl::<ResultLine>(&results_path)?
        .into_iter()
        .map(|line| (line.id, line.question_id))
        .collect();

    let mut pending = Vec::new();
    for event in &events {
        for qid in questions.keys() {
            if !done.contains(&(event.id.clone(), qid.clone())) {
                pending.push((event, qid.as_str()));
            }
        }
    }
    let Some(&(first, first_qid)) = pending.first() else {
        println!("nothing to do");
        return Ok(true);
    };

    // The first requests after idle are slow; their latency is not a result.
    let _ = ask(
        &args.endpoint,
        &first.state,
        first_qid,
        &questions[first_qid],
    );

    fs::create_dir_all(&args.data).map_err(|error| error.to_string())?;
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&results_path)
        .map_err(|error| error.to_string())?;
    let mut failures = 0;
    for (event, qid) in pending {
        let offset = client::log_len(&args.log);
        let started = Instant::now();
        let outcome = ask(&args.endpoint, &event.state, qid, &questions[qid]);
        let client_ms = started.elapsed().as_secs_f64() * 1000.0;
        let server_ms = outcome
            .is_ok()
            .then(|| client::server_ms_since(&args.log, offset));
        let mut line = ResultLine {
            backend: "decider".to_string(),
            id: event.id.clone(),
            question_id: qid.to_string(),
            question_version: version.clone(),
            input_tokens: None,
            client_ms,
            server_ms: server_ms.flatten(),
            answers: Answers::default(),
            error: None,
        };
        match outcome {
            Ok(answer) => {
                line.input_tokens = answer.input_tokens;
                match answer.value {
                    Reply::Noul { p_true } => {
                        line.answers.read_only = Some(ReadOnlyAnswer { p_true })
                    }
                    Reply::Choice {
                        choice,
                        probabilities,
                    } => {
                        line.answers.action = Some(ActionAnswer {
                            choice,
                            probabilities,
                        })
                    }
                }
            }
            Err(error) => line.error = Some(error),
        }
        failures += usize::from(line.error.is_some());
        let text = serde_json::to_string(&line).map_err(|error| error.to_string())?;
        writeln!(out, "{text}").map_err(|error| error.to_string())?;
        println!(
            "{} {qid} {}",
            event.id,
            line.error.as_deref().unwrap_or("ok")
        );
    }
    Ok(failures == 0)
}

fn main() -> ExitCode {
    let outcome = parse_args().and_then(|args| match args.command.as_str() {
        "run" => run(&args),
        "report" => report::write_report(&args.data).map(|()| true),
        other => Err(format!("unknown command {other}")),
    });
    match outcome {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("some requests failed");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
