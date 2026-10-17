//! The tree against the client fake daemon: the new client fields reach the rows, and loading
//! the tree sends no per-workspace checkout request.

#[path = "../../client/tests/common/mod.rs"]
mod common;

use bachuc_client::AttentionReason;
use bachuc_sidebar::model::{Facts, Group, Row, StatusBucket, ViewState, build_rows};
use common::{FakeServer, Harness};
use serde_json::{Value, json};
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(5);

fn workspace(id: &str, pinned_at: Value, diff_stat: Value) -> Value {
    json!({
        "id": id, "projectId": "p1", "projectDisplayName": "claude",
        "projectRootPath": "/repo/p1", "workspaceDirectory": format!("/repo/{id}"),
        "projectKind": "git", "workspaceKind": "worktree", "name": format!("ws {id}"),
        "pinnedAt": pinned_at, "diffStat": diff_stat, "status": "done", "activityAt": null,
        "gitRuntime": {"currentBranch": "main"}, "githubRuntime": null,
    })
}

#[test]
fn the_tree_loads_with_the_client_fields_and_no_checkout_request() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.with(|s| {
        s.agents[0]["workspaceId"] = json!("w1");
        s.agents[0]["requiresAttention"] = json!(true);
        s.agents[0]["attentionReason"] = json!("finished");
    });
    server.reply(
        "project.list.request",
        "project.list.response",
        json!({"projects": [{
            "projectId": "p1", "projectDisplayName": "claude", "projectRootPath": "/repo/p1",
            "projectKind": "git", "projectKey": "remote:github.com/hieppp/claude-setting",
        }]}),
    );
    server.reply(
        "fetch_workspaces_request",
        "fetch_workspaces_response",
        json!({"subscriptionId": "sub-w", "entries": [
            workspace("w1", Value::Null, json!({"additions": 12, "deletions": 3})),
            workspace("w2", json!("2026-10-16T00:00:00.000Z"), json!({"additions": 0, "deletions": 0})),
            workspace("w3", Value::Null, Value::Null),
        ], "pageInfo": {"nextCursor": null, "prevCursor": null, "hasMore": false}}),
    );
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected(), "connects");
    assert!(
        harness.pump_until(WAIT, |h| {
            h.core.workspace_diff_stat("w1").is_some() && h.core.project_view_key("p1").is_some()
        }),
        "the tree loads"
    );
    // Give a request the tree might send on load time to arrive.
    harness.pump_for(Duration::from_millis(300));

    let facts = Facts::read(
        &harness.core,
        harness.core.projects(),
        harness.core.agents(),
    );
    let rows = build_rows(
        harness.core.projects(),
        harness.core.agents(),
        &facts,
        &ViewState::default(),
    );
    let workspaces: Vec<_> = rows
        .iter()
        .filter_map(|row| match row {
            Row::Workspace(w) => Some(w),
            _ => None,
        })
        .collect();
    let find = |id: &str| *workspaces.iter().find(|w| w.id == id).expect(id);
    assert_eq!(find("w1").bucket, StatusBucket::Attention);
    assert_eq!(find("w1").diff, Some((12, 3)));
    assert_eq!(find("w2").diff, Some((0, 0)));
    assert_eq!(find("w2").group, Group::Pinned);
    assert_eq!(find("w3").diff, None);
    assert_eq!(
        harness.core.agent_attention("a1"),
        Some((true, Some(AttentionReason::Finished)))
    );
    let key = "remote:github.com/hieppp/claude-setting";
    assert_eq!(find("w1").color_key, key);
    assert!(
        rows.iter()
            .any(|row| matches!(row, Row::Project(p) if p.color_key == key))
    );

    assert!(
        server.requests_of("checkout_status_request").is_empty(),
        "no checkout.status request after the tree loads"
    );
}
