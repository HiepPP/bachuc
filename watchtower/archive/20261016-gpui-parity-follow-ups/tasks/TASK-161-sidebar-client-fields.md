# TASK-161 Sidebar on the new client fields

Group: E (writes `crates/sidebar`)
Class: code

## Brief

Goal: Make the sidebar show the Attention status, the Electron letter colors, the base-ref diff counts, and the pinned order (Q-074, Q-077, Q-078).

Change: no Attention status, letter colors hashed from the project id, diff counts from per-workspace `checkout_status` requests (`crates/sidebar/src/sidebar.rs` `fetch_diff` at line 261 and the call at line 381), pinned rows in tree order -> the new client accessors.

Design: [packages/protocol/src/agent-state-bucket.ts](packages/protocol/src/agent-state-bucket.ts) (lines 14-36), [packages/app/src/components/project-icon-view.tsx](packages/app/src/components/project-icon-view.tsx) (line 53), [packages/app/src/styles/identity-colors.ts](packages/app/src/styles/identity-colors.ts) (line 105), and the Electron sidebar's pinned sort (pinned rows by `pinnedAt`, newest first).

Boundaries:

- `bucket_for_agent` already takes `requires_attention` and `attention_reason` (`crates/sidebar/src/model.rs` lines 56-70); pass the values of `agent_attention`.
- Hash `Client::project_view_key` with the existing `identity_color`; fall back to the project id only when the accessor returns `None`.
- Show `workspace_diff_stat` as Electron does, including how Electron shows a `0/0` stat; remove both per-workspace `checkout_status` calls. `None` shows no count.
- Keep `Sidebar::new`, `set_selected_workspace`, and `SidebarEvent` unchanged. Projects still sort by name (Q-085).

Files:

- [packages/desktop-gpui/crates/sidebar](packages/desktop-gpui/crates/sidebar) (`src`, `tests`)

Interfaces:

- Consumes: `bachuc_client::AttentionReason` (enum `Finished`, `Error`, `Permission`) and `Client::agent_attention(&self, agent_id: &str) -> Option<(bool, Option<AttentionReason>)>` (requires attention, reason) from TASK-157
- Consumes: `Client::project_view_key(&self, project_id: &str) -> Option<String>` from TASK-157
- Consumes: `bachuc_client::DiffStat` (struct `additions: u32`, `deletions: u32`), `Client::workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat>`, and `Client::workspace_pinned_at(&self, workspace_id: &str) -> Option<String>` from TASK-157
- Produces: the same `bachuc_sidebar::Sidebar` and `SidebarEvent` API for TASK-167

Expected result:

- A workspace with an agent that needs attention shows the Attention status.
- The `.claude` project letter box uses the `projectKey` color.

Anti-goal: the sidebar sends no `checkout_status` request at load; tripwire: a fake-daemon test counts zero `checkout.status` requests after the tree loads; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-sidebar` -> pass, including tests for the Attention bucket, the `projectKey` color and the fallback, a `None` and a `0/0` diff stat, the pinned order by `pinned_at` with equal and missing values, and the zero-request test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-sidebar` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
