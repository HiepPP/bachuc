# TASK-271 outcome

Status: DONE (screen check deferred to TASK-272)

Changed:

- `packages/desktop-gpui/crates/timeline/src/lib.rs`: `read_only` and `TimelineView::read_only`, the child running rule, the hidden fork, rewind, and permission parts, the skipped prompt index, and the public rules `subagent_turn_active`, `shows_fork`, `shows_rewind`, `shows_permissions`.
- `packages/desktop-gpui/crates/timeline/tests/read_only.rs` (new): the running rule and the action rule.

Contract:

- `TimelineView::read_only(client: Entity<Client>, key: String, diagrams: Entity<DiagramRenderer>, window: &mut Window, cx: &mut Context<TimelineView>) -> TimelineView`.
- The child runs when its row in `Client::subagents(parent)` has status `running`; a completed, failed, or missing row is finished. The parent and child come from `parse_provider_subagent_key`.
- Read-only hides the fork menu in every footer, the rewind menu under user messages, and permission rows. Copy and links stay.
- `list_prompts` is skipped, so no daemon request carries the key as `agentId`. `timeline_epoch`, `agent_labels`, and `rewind_modes` are safe for a key (none, none, empty).

Verified:

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline`: PASS (group-level run; 136 tests, 0 failed).
- `tests/read_only.rs` running rule: `running` gives a running turn; `completed`, `failed`, and a missing row give a finished turn: PASS.
- `tests/read_only.rs` action rule: read-only hides fork, rewind, and permission rows; agent mode keeps them: PASS.
- Read check: in read-only mode `view_agent(key)` and `load_older(key)` send child requests (TASK-270), `refresh_prompts` returns before `list_prompts`, `shows_rewind` suppresses the rewind menu, `shows_permissions` empties the permission list, and `fork_boundary_for` returns `None`. No other daemon call carries the key as `agentId`; a Stop cannot appear because a key has no background processes.
- Screen check: opening the view is TASK-272 -> DEFERRED (UI checker).
- Anti-goal: the old test count before changes was 119 (`git grep -c "#\[test\]" HEAD` over the crate); all 119 still pass, and the crate now has 136 tests.
- Anti-goal: the normal agent timeline keeps the same rows and actions; the read checks above cover each changed call site, and every old `bachuc-timeline` test passes.

Anti-goal: 119 old tests before -> 119 old tests still passing, plus 17 new tests (6 background, 9 motion, 2 read_only).

Lessons:

- A provider subagent key is not in `agents()` and carries no extras, so `turn_active`, `agent_labels`, and `rewind_modes` must not read it as an agent id. The running state comes from the parent's subagent rows.
- The prompt index and the outline rail stay off in read-only mode; `outline_enabled` returns false so the render never defers a prompt fetch.
- Fork, rewind, and permissions branch on the `read_only` flag, not on a missing feature.
