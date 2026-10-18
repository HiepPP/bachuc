# TASK-219 Composer draft seed and raw attachment

Group: M (standalone: [packages/desktop-gpui/crates/composer](packages/desktop-gpui/crates/composer))
Class: code

## Brief

Goal: A draft composer can be seeded with a source agent's provider, model, mode, thinking option, and features, and can carry a raw attachment such as a fork's chat history.

Change: a draft only takes a cwd (`set_draft_cwd`, `composer.rs` line 522) and attachments come only from plugin chips -> a seed setter and a raw attachment chip.

Boundaries:

- Today `DraftOptions` ([packages/desktop-gpui/crates/composer/src/draft.rs](packages/desktop-gpui/crates/composer/src/draft.rs) lines 24-29) has cwd, lines, icon, and empty-submit only. The send path already carries raw JSON attachments (`draft.rs` line 47).
- Plugin chips come from `composer.rs` lines 586-597 and are turned into agent attachments by `to_agent_attachment` ([packages/desktop-gpui/crates/plugin-host/src/composer.rs](packages/desktop-gpui/crates/plugin-host/src/composer.rs) lines 238-262), which emits no `contextKind`. Do not change that path. Add a separate raw attachment that the composer sends unchanged.
- The raw attachment shows as a removable chip with its label. Removing it drops it from the send.
- The seed sets the draft's provider, model, mode, thinking option, and feature values as if the user picked them. A seed field the provider does not offer is ignored.
- Electron reference: [packages/app/src/hooks/use-fork-agent.ts](packages/app/src/hooks/use-fork-agent.ts) lines 79-111.
- This TASK runs after TASK-214, which also edits `composer.rs`.

How:

- Add `DraftSeed` and `Composer::set_draft_seed`.
- Add `Composer::add_raw_attachment` and the chip.
- Put "which seed fields apply for this provider" in a pure function with a test.

Files:

- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (seed setter, raw attachment chip, send)
- [packages/desktop-gpui/crates/composer/src/draft.rs](packages/desktop-gpui/crates/composer/src/draft.rs) (seed fields, raw attachments in the send payload)

Interfaces:

- Produces: `DraftSeed { provider: Option<String>, model: Option<String>, mode: Option<String>, thinking_option_id: Option<String>, features: Option<serde_json::Value> }` for TASK-201.
- Produces: `Composer::set_draft_seed(&mut self, seed: DraftSeed, window: &mut Window, cx: &mut Context<Composer>)` for TASK-201.
- Produces: `Composer::add_raw_attachment(&mut self, label: String, attachment: serde_json::Value, cx: &mut Context<Composer>)` for TASK-201.

Expected result:

- A seeded draft shows the source provider and model in its pills.
- A raw attachment shows as a chip and is sent unchanged with the first prompt.
- A draft with no raw attachment sends the same payload as before.

Anti-goal: a normal draft send must not change; tripwire; a unit test of the send payload with no seed and no raw attachment equals the payload before the change, read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer` -> passes, including the seed rule test and the unchanged payload test.
- `./cargo.sh build -p bachuc-app` -> builds.
- Anti-goal: the unchanged payload test -> passes at both reads.
