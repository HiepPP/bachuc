//! Prints the agent list of a host, then the live timeline of one agent.
//!
//! Usage: client_probe [ws url] [agent id or id prefix] [seconds]
//! Without an agent it stays connected and idle. With seconds it quits after that long.
//! A password comes from `BACHUC_PASSWORD`. `BACHUC_WATCH_CHECKOUT=<folder>` watches that
//! folder's changed files. The project tree is always kept.

use bachuc_client::{Client, ClientEvent, DirectoryEvent, HostConfig};
use gpui_kit::*;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[derive(Default)]
struct Probe {
    listed: bool,
    agent: Option<String>,
    printed: Vec<String>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "ws://127.0.0.1:6768/ws".into());
    let wanted = args.next();
    let seconds: Option<u64> = args.next().and_then(|s| s.parse().ok());
    gpui_kit::application().run(move |cx: &mut App| {
        let host = HostConfig {
            name: "probe".into(),
            url,
            password: std::env::var("BACHUC_PASSWORD").ok(),
        };
        let client = Client::connect(host, cx);
        let probe = Rc::new(RefCell::new(Probe::default()));
        // A subscription holds its emitter weakly; this handle keeps the client alive.
        let keep_alive = client.clone();
        cx.subscribe(&client, move |client, event: &ClientEvent, cx| {
            let _ = &keep_alive;
            on_event(&probe, &wanted, client, event, cx);
        })
        .detach();
        cx.subscribe(&client, |client, event: &DirectoryEvent, cx| match event {
            DirectoryEvent::ProjectsChanged => {
                println!("{} projects", client.read(cx).projects().len())
            }
            DirectoryEvent::CheckoutChanged { cwd } => println!("checkout changed: {cwd}"),
        })
        .detach();
        if let Some(seconds) = seconds {
            cx.spawn(async move |cx| {
                cx.background_executor()
                    .timer(Duration::from_secs(seconds))
                    .await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });
}

fn on_event(
    probe: &Rc<RefCell<Probe>>,
    wanted: &Option<String>,
    client: Entity<Client>,
    event: &ClientEvent,
    cx: &mut App,
) {
    match event {
        ClientEvent::Connected => {
            println!("connected");
            if let Ok(cwd) = std::env::var("BACHUC_WATCH_CHECKOUT") {
                println!("watching checkout {cwd}");
                client.update(cx, |client, cx| client.watch_checkout(&cwd, cx));
            }
        }
        ClientEvent::Disconnected { reason } => println!("disconnected: {reason}"),
        ClientEvent::AgentsChanged => {
            if probe.borrow().listed || client.read(cx).agents().is_empty() {
                return;
            }
            probe.borrow_mut().listed = true;
            let agents = client.read(cx).agents().to_vec();
            println!("{} agents", agents.len());
            for agent in &agents {
                println!(
                    "  {} {:<8} {:<8} {}",
                    &agent.id[..8.min(agent.id.len())],
                    agent.status,
                    agent.provider,
                    agent.title
                );
            }
            let Some(wanted) = wanted else {
                return;
            };
            let Some(agent) = agents.iter().find(|a| a.id.starts_with(wanted.as_str())) else {
                println!("agent {wanted} not found");
                return;
            };
            println!("viewing {}", agent.id);
            probe.borrow_mut().agent = Some(agent.id.clone());
            let id = agent.id.clone();
            client.update(cx, |client, cx| client.view_agent(&id, cx));
        }
        ClientEvent::TimelineChanged { agent_id } => {
            if probe.borrow().agent.as_deref() != Some(agent_id.as_str()) {
                return;
            }
            let client = client.read(cx);
            let Some(timeline) = client.timeline(agent_id) else {
                return;
            };
            let mut probe = probe.borrow_mut();
            for (index, entry) in timeline.entries.iter().enumerate() {
                let line = summary(entry);
                match probe.printed.get(index) {
                    Some(old) if *old == line => {}
                    Some(old) if line.starts_with(old.as_str()) => {
                        println!("#{index} +{}", &line[old.len()..]);
                        probe.printed[index] = line;
                    }
                    _ => {
                        println!("#{index} {line}");
                        if index < probe.printed.len() {
                            probe.printed[index] = line;
                        } else {
                            probe.printed.push(line);
                        }
                    }
                }
            }
        }
        ClientEvent::PermissionRequested { agent_id } => {
            println!("permission requested: {agent_id}")
        }
        ClientEvent::AgentFinished { agent_id } => println!("finished: {agent_id}"),
    }
}

/// One line per entry: its type and text, or a tool call's name and status.
fn summary(entry: &Value) -> String {
    let item = &entry["item"];
    let kind = item["type"].as_str().unwrap_or("?");
    match kind {
        "tool_call" => format!(
            "[tool_call] {} {}",
            item["name"].as_str().unwrap_or("?"),
            item["status"].as_str().unwrap_or("")
        ),
        _ => format!(
            "[{kind}] {}",
            item["text"].as_str().unwrap_or("").replace('\n', " ")
        ),
    }
}
