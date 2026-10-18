# TASK-219 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/src/draft.rs`: `DraftSeed`, the seed rules `seed_provider_model`, `offered`, and `seed_feature_values`, and `outgoing_attachments`. It adds 3 tests: the seed rule test, the raw attachment order test, and the tripwire `a_plain_draft_payload_is_unchanged`.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: `Composer::set_draft_seed`, `Composer::add_raw_attachment`, the raw attachment chip (`raw_chip`), and the raw attachments in the draft submission and the agent send.
- `packages/desktop-gpui/crates/composer/src/lib.rs`: exports `DraftSeed`. This file is not in the spec's Files. The `draft` module is private, so TASK-201 cannot name `DraftSeed` without this one-word change.

## Contract

- Produces `DraftSeed { provider, model, mode, thinking_option_id: Option<String>, features: Option<serde_json::Value> }`, exported as `bachuc_composer::DraftSeed`. It derives `Clone, Debug, Default, PartialEq`. `features` is a JSON object of feature values by feature id, as `buildForkDraftSetup` builds `featureValues`.
- Produces `Composer::set_draft_seed(&mut self, seed: DraftSeed, window: &mut Window, cx: &mut Context<Composer>)`. On an agent composer it does nothing.
  - Provider: the seed's provider when it is in `client.providers()` and available. A seed with no provider keeps the draft's provider. If the provider does not apply, the whole seed is ignored, because its other fields belong to that provider.
  - Model: the seed's model when the provider lists it, else the provider's default model.
  - Thinking: the seed's option when `model_thinking(provider, model)` offers it, else none (the model default).
  - Mode and features: kept as pending, and applied when the next accepted `provider_modes` and `draft_features` answers arrive. A mode not in the list and a feature value whose id, type, or select option does not match are ignored. A model pick of another provider drops the pending mode and features.
  - The seed then calls `reload_draft`, so a draft with no `cwd` keeps the pending mode and features until `set_draft_cwd` gives it a folder. TASK-201 can call `set_draft_seed` and `set_draft_cwd` in either order.
- Produces `Composer::add_raw_attachment(&mut self, label: String, attachment: serde_json::Value, cx: &mut Context<Composer>)`. It works on a draft and on an agent composer.
  - The chip shows a `FileText` glyph and the label, with a remove button (`Role::Button`, "Remove attachment", click and `AccessibleAction::Click`). Removing it drops it from the send.
  - The send puts the raw attachments first, unchanged, then the plugin chips (`outgoing_attachments`). `draft_submission().attachments` and the agent `OutgoingMessage.attachments` both carry them. The plugin chip path (`to_agent_attachment`) did not change.
  - An agent send takes the raw attachments and puts them back if the send fails, as it does for plugin chips. `Composer::clear` drops them.
  - A raw attachment counts as an attachment for `can_send`, so a prompt with only a raw attachment can send.
- A draft with no seed and no raw attachment sends the same payload as before. `DraftSubmission` and `DraftSubmission::build` did not change.

## Verified

- Anti-goal read 1, before the change: `./cargo.sh test -p bachuc-composer --lib a_plain_draft_payload_is_unchanged` -> 1 passed.
- Anti-goal read 2, at completion: `./cargo.sh test -p bachuc-composer --lib draft::` -> 9 passed, including `a_plain_draft_payload_is_unchanged`. The test body did not change between the reads.
- `./cargo.sh test -p bachuc-composer` -> 113 passed (110 before, plus 3 new), including `a_seed_field_the_provider_does_not_offer_is_ignored`.
- `./cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `./cargo.sh clippy -p bachuc-composer --tests` -> no warnings in `composer.rs` or `draft.rs`. The warnings it gives are in other files and existed before.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- `./cargo.sh build -p bachuc-app` -> exit 0.
- Screen checks (a seeded draft shows the source provider and model in its pills, and the chip shows) -> DEFERRED (UI checker).

## Lessons

- `set_draft_seed` reads the provider list the composer already holds. A seed given before the client has its provider list is ignored. Providers arrive with the connection, so a fork from a connected agent has them.
- The seed sets `mode_id` to none at once. The seeded mode shows only after `provider_modes` answers, and only when the draft has a `cwd`.
- For TASK-201: call `add_raw_attachment(title, fork_context.attachment, cx)` with the chat history label (Electron uses `attachment.title`, or "Text attachment"). The composer sends the JSON as given, so `contextKind: "chat_history"` stays in it.

## UI Checks Run 4

Run 2026-10-18 by the UI checker (run 4). Bundle `Bachuc GPUI Check` built from HEAD `60fd3a593`, PID 88032, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The checker forced each frame with the AX "zoom the window" action and sent keys with cua `pressKey`/`typeText` to the check app only. Scratch repo `/tmp/t202-repo` with disposable agents "r4 board card", "r4 remove card", "r4 fork src" (mock ten-second-stream), "r4 fork ok" (Haiku 5.5, one-word reply), and the fork child "Continue." (`af3218a`). After the run, PID 88032 exited, all five agents and the five t202-repo workspaces were archived, and the repo stayed as before (`feature-x`, ` M a.txt`).

- A seeded draft shows the source provider and model in its pills, and the chip shows: PASS, covered by the TASK-201 fork check. Both fork targets showed Haiku 5.5 (Claude logo), Medium, Always Ask, and the "Chat history" attachment chip. Evidence: `/tmp/r4-201-08-fork-new-tab.png`, `/tmp/r4-201-11-fork-new-ws.png`.
