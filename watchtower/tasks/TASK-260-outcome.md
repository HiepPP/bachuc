# TASK-260 Outcome

## Status

DONE.

## Changed

- `packages/desktop-gpui/crates/client/src/model.rs`: `RewindMode` (`Conversation`, `Files`, `Both`; private `wire`) and `RewindModes { conversation, files, both }`.
- `packages/desktop-gpui/crates/client/src/core.rs`: `upsert_agent` reads `capabilities.supportsRewind*` into `AgentExtra.rewind`; `ClientCore::rewind_modes` and `ClientCore::rewind`; `Pending::Rewind`; the `agent.rewind.response` arm.
- `packages/desktop-gpui/crates/client/src/lib.rs`: `Client::rewind_modes`, `Client::rewind`, and the `RewindMode`, `RewindModes` exports.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: 4 new tests.

## Contract

- `Client::rewind_modes(&self, agent_id: &str) -> RewindModes`: as specified. All false for a host without `features.rewind`, an unknown agent, or flags that are absent.
- `Client::rewind(&mut self, agent_id: &str, message_id: &str, mode: RewindMode, cx: &mut Context<Client>) -> Task<Result<(), String>>`. This differs from the spec line (`&self`, `cx: &mut App`) because every `Client` call that sends a request needs `&mut self` and the entity `Context` (like `fork_context`); a `&self` method cannot send. Call it as `client.update(cx, |client, cx| client.rewind(..., cx))`. `Err` holds the host error text, "Rewind failed" for `ok: false` with no text, or "Update the host to use this." without the `rewind` feature.
- After `ok: true` with `Conversation` or `Both`, the client reloads the timeline tail (`plan_tail`), as for `agent.timeline.replacement`. `Files` reloads nothing.
- Wire: `{type: "agent.rewind.request", agentId, messageId, mode, requestId}`; answer `agent.rewind.response` `payload {requestId, agentId, ok, error}`.

## Verified

- `./cargo.sh test -p bachuc-client` -> all suites pass (rpcs 65).
- New tests: request payload matches the wire shape; `ok: true` resolves `Ok` and reloads the tail (Conversation) or not (Files); `ok: false` resolves `Err` with the text, the timeline entries equal the entries before (2 = 2) and no tail request goes out; `ok: false` without text gives "Rewind failed"; no `rewind` feature gives all modes false and sends nothing; modes follow the agent flags.
- `./cargo.sh clippy -p bachuc-client --all-targets`, `./cargo.sh fmt --check -p bachuc-client` -> clean.

## Anti-goal

The error path (`a_failed_rewind_resolves_with_the_error_and_changes_no_client_state`) reads the timeline entries before (2) and after (2, equal) and the tail-fetch request count before and after (equal); read in the test. The only state change on a reply is in the `result.is_ok()` branch.

## Lessons

- The task spec signature for `Client::rewind` could not be matched literally; TASK-263 must call it through `Entity::update`.
