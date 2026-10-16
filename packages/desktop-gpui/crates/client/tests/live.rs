//! Checks against the live daemon on port 6768. Run with `-- --ignored live_`.
//! They create a mock agent and terminals, then stop, archive, and kill them.

mod common;

use bachuc_client::{ClientEvent, TerminalFrame, TerminalHandle};
use common::Harness;
use futures::channel::oneshot;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const LIVE: &str = "ws://127.0.0.1:6768/ws";
const BENCH: &str = "/tmp/gpui-bench";
const WAIT: Duration = Duration::from_secs(20);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

/// Runs `npm run cli -- <args>` against live; `scripts/dev-home.sh` picks the live home.
fn cli(args: &[&str]) {
    let output = Command::new("npm")
        .args(["run", "--silent", "cli", "--"])
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("npm runs");
    if !output.status.success() {
        eprintln!(
            "cli {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

struct AgentCleanup(String);

impl Drop for AgentCleanup {
    fn drop(&mut self) {
        cli(&["stop", &self.0]);
        cli(&["archive", &self.0]);
    }
}

struct TerminalCleanup(String);

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        cli(&["terminal", "kill", &self.0]);
    }
}

fn wait_reply<T>(harness: &mut Harness, mut reply: oneshot::Receiver<anyhow::Result<T>>) -> T {
    let mut result = None;
    assert!(harness.pump_until(WAIT, |_| {
        if let Ok(Some(value)) = reply.try_recv() {
            result = Some(value);
        }
        result.is_some()
    }));
    result.unwrap().expect("request succeeds")
}

fn next_frames(
    harness: &mut Harness,
    handle: &mut TerminalHandle,
    until: impl Fn(&[TerminalFrame]) -> bool,
) -> Vec<TerminalFrame> {
    let mut frames = Vec::new();
    harness.pump_until(WAIT, |_| {
        while let Ok(frame) = handle.frames.try_recv() {
            frames.push(frame);
        }
        until(&frames)
    });
    frames
}

/// The text of a snapshot grid and its scrollback.
fn snapshot_text(state: &Value) -> String {
    let rows = state["scrollback"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(state["grid"].as_array().into_iter().flatten());
    rows.map(|row| {
        row.as_array()
            .into_iter()
            .flatten()
            .map(|cell| cell["char"].as_str().unwrap_or(" "))
            .collect::<String>()
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn output_text(frames: &[TerminalFrame]) -> String {
    frames
        .iter()
        .filter_map(|frame| match frame {
            TerminalFrame::Output(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
            _ => None,
        })
        .collect()
}

/// Projected fields that a live merge and a fresh fetch must agree on.
fn comparable(entry: &Value) -> Value {
    let mut entry = entry.clone();
    if let Some(map) = entry.as_object_mut() {
        map.remove("turnMetrics");
    }
    entry
}

#[test]
#[ignore]
fn live_stream_lists_agents_and_matches_a_fresh_fetch() {
    let mut client = Harness::connect(LIVE, None);
    assert!(client.connected(), "live daemon on 6768");
    let reply =
        client
            .core
            .create_agent("mock", Some("thirty-minute-stream"), BENCH, "start bench");
    let agent_id = wait_reply(&mut client, reply);
    let _cleanup = AgentCleanup(agent_id.clone());
    assert!(client.pump_until(WAIT, |h| h.core.agents().iter().any(|a| a.id == agent_id)));
    client.core.view_agent(&agent_id);
    // About 12 s covers an assistant message, reasoning, and the first tool calls.
    client.pump_until(Duration::from_secs(25), |h| {
        h.entries(&agent_id)
            .iter()
            .any(|e| e["item"]["type"] == "tool_call")
    });
    client.pump_for(Duration::from_secs(1));
    let changes = client
        .events
        .iter()
        .filter(|e| {
            **e == ClientEvent::TimelineChanged {
                agent_id: agent_id.clone(),
            }
        })
        .count();
    // The mock's chunks merge into few entries.
    let streamed = client.entries(&agent_id);
    let kinds: Vec<&str> = streamed
        .iter()
        .filter_map(|e| e["item"]["type"].as_str())
        .collect();
    for kind in [
        "user_message",
        "assistant_message",
        "reasoning",
        "tool_call",
    ] {
        assert!(kinds.contains(&kind), "streamed {kind}: {kinds:?}");
    }
    assert!(changes >= 10, "timeline changes: {changes}");

    client.core.cancel_agent(&agent_id);
    assert!(client.pump_until(WAIT, |h| {
        h.core
            .agents()
            .iter()
            .any(|a| a.id == agent_id && a.status != "running")
    }));
    client.pump_for(Duration::from_secs(1));

    let mut fresh = Harness::connect(LIVE, None);
    assert!(fresh.connected());
    fresh.core.view_agent(&agent_id);
    assert!(fresh.pump_until(WAIT, |h| !h.entries(&agent_id).is_empty()));
    fresh.pump_for(Duration::from_millis(500));
    let mine = client.entries(&agent_id);
    let theirs = fresh.entries(&agent_id);
    let first = |entries: &[Value]| {
        entries
            .first()
            .and_then(|e| e["seqStart"].as_u64())
            .unwrap()
    };
    let from = first(&mine).max(first(&theirs));
    let overlap = |entries: &[Value]| -> Vec<Value> {
        entries
            .iter()
            .filter(|e| e["seqStart"].as_u64().unwrap() >= from)
            .map(comparable)
            .collect()
    };
    let (mine, theirs) = (overlap(&mine), overlap(&theirs));
    assert!(mine.len() >= 2);
    for (index, (a, b)) in mine.iter().zip(&theirs).enumerate() {
        assert_eq!(a, b, "entry {index} differs");
    }
    assert_eq!(mine.len(), theirs.len());
}

#[test]
#[ignore]
fn live_terminal_attach_gets_a_snapshot_first() {
    let mut owner = Harness::connect(LIVE, None);
    assert!(owner.connected(), "live daemon on 6768");
    assert!(owner.pump_until(WAIT, |h| h.core.workspaces().iter().any(|w| w.cwd == BENCH)));
    let reply = owner.core.create_terminal(BENCH);
    let mut first = wait_reply(&mut owner, reply);
    let _cleanup = TerminalCleanup(first.id.clone());
    let frames = next_frames(&mut owner, &mut first, |f| !f.is_empty());
    assert!(
        matches!(frames[0], TerminalFrame::Snapshot(_)),
        "create starts with a snapshot"
    );
    owner.core.terminal_input(&first.id, b"echo attach-check\r");
    let frames = next_frames(&mut owner, &mut first, |f| {
        output_text(f).contains("attach-check\r\n")
    });
    assert!(output_text(&frames).contains("attach-check"));

    let mut viewer = Harness::connect(LIVE, None);
    assert!(viewer.connected());
    let reply = viewer.core.attach_terminal(&first.id);
    let mut attached = wait_reply(&mut viewer, reply);
    let frames = next_frames(&mut viewer, &mut attached, |f| !f.is_empty());
    let TerminalFrame::Snapshot(state) = &frames[0] else {
        panic!("attach starts with a snapshot, got {:?}", frames[0]);
    };
    assert!(
        snapshot_text(state).contains("attach-check"),
        "snapshot holds earlier output"
    );
    viewer
        .core
        .terminal_input(&attached.id, b"echo after-attach\r");
    let frames = next_frames(&mut viewer, &mut attached, |f| {
        output_text(f).contains("after-attach\r\n")
    });
    assert!(
        output_text(&frames).contains("after-attach"),
        "output follows the snapshot"
    );
    assert!(frames.iter().all(|f| matches!(f, TerminalFrame::Output(_))));

    viewer.core.kill_terminal(&attached.id);
    viewer.pump_for(Duration::from_millis(500));
}

/// A 1x1 PNG.
const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

/// A temporary git folder with one committed and changed text file and an image.
struct TempRepo(PathBuf);

impl TempRepo {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("gpui-146-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        // `temp_dir` can be a symlink; the daemon answers with the real path.
        let dir = dir.canonicalize().unwrap();
        std::fs::write(dir.join("a.txt"), "one\ntwo\n").unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", "init"]);
        std::fs::write(dir.join("a.txt"), "one\nchanged\n").unwrap();
        use base64::Engine as _;
        let png = base64::engine::general_purpose::STANDARD
            .decode(PNG)
            .unwrap();
        std::fs::write(dir.join("img.png"), png).unwrap();
        Self(dir)
    }

    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore]
fn live_projects_files_and_checkout() {
    let mut client = Harness::connect(LIVE, None);
    assert!(client.connected(), "live daemon on 6768");
    assert!(
        client.pump_until(WAIT, |h| h
            .core
            .projects()
            .iter()
            .any(|p| !p.workspaces.is_empty())),
        "projects load"
    );
    assert!(
        client.core.typed_server_info().is_some(),
        "server_info decodes"
    );

    let repo = TempRepo::new();
    let reply = client.core.list_directory(repo.path(), ".");
    let entries = client.wait(reply, WAIT).expect("directory lists");
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"a.txt") && names.contains(&"img.png"),
        "{names:?}"
    );

    let reply = client.core.read_file(repo.path(), "a.txt");
    let text = client.wait(reply, WAIT).expect("text file reads");
    assert_eq!(text.kind, bachuc_client::FileKind::Text);
    assert_eq!(text.text.as_deref(), Some("one\nchanged\n"));

    let reply = client.core.read_file(repo.path(), "img.png");
    let image = client.wait(reply, WAIT).expect("image reads");
    assert_eq!(image.kind, bachuc_client::FileKind::Image);
    assert_eq!(image.mime_type.as_deref(), Some("image/png"));
    let png = std::fs::read(repo.0.join("img.png")).unwrap();
    assert_eq!(image.bytes.as_deref(), Some(png.as_slice()));

    let reply = client.core.checkout_status(repo.path());
    let status = client.wait(reply, WAIT).expect("checkout status");
    assert_eq!(status.branch.as_deref(), Some("main"));
    let changed = status
        .files
        .iter()
        .find(|f| f.path == "a.txt")
        .expect("a.txt changed");
    assert_eq!(
        (changed.status.as_str(), changed.added, changed.removed),
        ("ok", 1, 1)
    );

    let reply = client.core.file_diff(repo.path(), "a.txt");
    let diff = client.wait(reply, WAIT).expect("file diff");
    assert!(diff.contains("-two\n+changed\n"), "{diff}");

    let plain = std::env::temp_dir().join(format!("gpui-146-plain-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&plain).unwrap();
    let reply = client.core.checkout_status(plain.to_str().unwrap());
    let status = client.wait(reply, WAIT);
    let _ = std::fs::remove_dir_all(&plain);
    assert_eq!(
        status.expect("non-git status"),
        bachuc_client::CheckoutStatus {
            branch: None,
            files: vec![]
        }
    );

    let changed = bachuc_client::DirectoryEvent::CheckoutChanged {
        cwd: repo.path().to_string(),
    };
    client.core.watch_checkout(repo.path());
    assert!(
        client.pump_until(WAIT, |h| h.directory_events.contains(&changed)),
        "watch starts"
    );
    client.directory_events.clear();
    std::fs::write(repo.0.join("b.txt"), "new file\n").unwrap();
    std::fs::write(repo.0.join("a.txt"), "one\nchanged again\n").unwrap();
    assert!(
        client.pump_until(WAIT, |h| h.directory_events.contains(&changed)),
        "a change on disk emits CheckoutChanged"
    );
    client.core.unwatch_checkout(repo.path());
    client.pump_for(Duration::from_millis(300));
}

#[test]
#[ignore]
fn live_agent_commands_thinking_and_queue() {
    let mut client = Harness::connect(LIVE, None);
    assert!(client.connected(), "live daemon on 6768");
    assert!(client.core.supports_queue(), "live has agentMessageQueue");
    // `five-minute-stream` has thinking options and runs long enough to queue behind.
    let reply = client
        .core
        .create_agent("mock", Some("five-minute-stream"), BENCH, "start bench");
    let agent_id = wait_reply(&mut client, reply);
    let _cleanup = AgentCleanup(agent_id.clone());
    let running = client.pump_until(WAIT, |h| {
        h.core
            .agents()
            .iter()
            .any(|a| a.id == agent_id && a.status == "running")
    });
    let status = client
        .core
        .agents()
        .iter()
        .find(|a| a.id == agent_id)
        .map(|a| a.status.clone());
    assert!(running, "agent runs, status {status:?}");

    // The mock provider lists no commands; an idle Claude agent with no prompt runs no turn.
    let reply = client.core.list_commands(&agent_id);
    let error = client.wait(reply, WAIT).expect_err("mock has no commands");
    assert!(error.to_string().contains("does not support"), "{error}");
    let reply = client.core.create_agent("claude", None, BENCH, "");
    let claude_id = wait_reply(&mut client, reply);
    let _claude_cleanup = AgentCleanup(claude_id.clone());
    let reply = client.core.list_commands(&claude_id);
    let commands = client
        .wait(reply, Duration::from_secs(60))
        .expect("claude lists commands");
    assert!(!commands.is_empty(), "claude has slash commands");

    assert!(
        client.pump_until(WAIT, |h| h.core.agent_thinking(&agent_id).is_some()),
        "thinking"
    );
    let thinking = client.core.agent_thinking(&agent_id).unwrap();
    let other = thinking
        .options
        .iter()
        .map(|(id, _)| id.clone())
        .find(|id| Some(id) != thinking.current.as_ref())
        .expect("a second thinking option");
    client.core.set_thinking(&agent_id, &other);
    assert!(
        client.pump_until(WAIT, |h| {
            h.core.agent_thinking(&agent_id).and_then(|t| t.current) == Some(other.clone())
        }),
        "thinking changes to {other}"
    );

    client
        .core
        .enqueue_prompt(&agent_id, "queued while running".into(), vec![]);
    assert!(
        client.pump_until(WAIT, |h| h
            .core
            .queued_prompts(&agent_id)
            .iter()
            .any(|q| q.text == "queued while running")),
        "prompt is queued"
    );
    let queued = client.core.queued_prompts(&agent_id);
    let item = queued
        .iter()
        .find(|q| q.text == "queued while running")
        .unwrap();
    client.core.cancel_queued(&agent_id, &item.id);
    assert!(
        client.pump_until(WAIT, |h| h.core.queued_prompts(&agent_id).is_empty()),
        "queued prompt is cancelled"
    );
}
