# TASK-013 Inline prompt Send and optional Jev auto-send

Group: P (standalone plugin and catalog)
Class: risky

## Brief

Goal: Send a suggested next prompt directly from its block, without copying into the composer.
Offer optional Jev evaluation and automatic sending within the user's authorized task.

Change: Add a standalone next-prompt-actions plugin. Keep Paseo application source unchanged.

How:

- Check the installed host version and supported timeline rendering, message identity, and prompt-send APIs first.
- Prove that an extension can place Send inside the original block without replacing unrelated message content.
- If unsupported, report the exact missing extension point before implementation. Do not silently use private DOM injection.
- Detect fenced blocks with `prompt:` entries under `What Next` or `Next Steps` in completed assistant messages.
- Ignore user messages, tool output, ordinary code blocks, empty prompts, and incomplete streamed messages.
- Give each prompt its own Send action. Remove only its leading `prompt:` marker when sending.
- Preserve remaining text, newlines, and Vietnamese diacritics. Never send the heading or explanation.
- Send to the source conversation through the supported API. Preserve any unsent composer draft.
- Keep Send inside the block: in a bottom-right footer at every width, after all prompt text. Avoid text overlap.
- Keep the Jev setting outside prompt blocks.
- Keep copy usable. Include keyboard focus, accessible labels, and Send, Sending, Sent, and Retry states.
- Prevent duplicate sends across rapid clicks, auto-send races, rerenders, reconnects, and multiple clients.
- Bind deduplication to conversation, message, and prompt identity. Do not retry an ambiguous send automatically.
- Reject stale actions when the conversation changed or a newer user turn arrived. Disable sending while the agent is busy.
- Add a persisted per-conversation `Auto-send with Jev` toggle, disabled by default and available in the agent Command Center.
- When enabled, evaluate only new completed responses. Do not execute historical prompts after enabling or reloading.
- Use the existing Jev evaluation contract with minimal relevant task context and explicit allowed decisions.
- Jev checks relevance, unmet prerequisites, duplicate work, completion, and whether user input is required.
- Jev is an evaluator, not a coding provider. Reuse an existing supported integration; avoid a new delegation framework.
- Send only on a valid positive decision plus deterministic authorization and freshness checks.
- Errors, timeout, unavailable Jev, missing evidence, or ambiguous choices leave manual Send available.
- Auto-send cannot expand task scope or bypass approval requirements. Show evaluation and send status separately.
- Bound each auto-send chain to three turns and three evaluations. Stop on completion, error, limit, or user interruption.
- Turning the toggle off cancels pending automation. Recheck the toggle immediately before dispatch.
- Keep evaluation input, raw prompts, and deduplication state outside Git. Do not log full prompt content.
- Use the generated imagegen concept for button styling and placement; it is a proposal, not runtime evidence.
- Document settings, compatibility, integration limits, and cleanup. Add the plugin to the root catalog.

Files:

- [New plugin](../../../../plugins/next-prompt-actions) contains its manifest, entries, client, server, shared contracts, tests, and README.
- [Catalog](../../../../README.md) adds the plugin after implementation.
- [Design concept](../../../designs/TASK-013-next-prompt-actions.png) shows desktop, narrow layout, and button states.
- [Jev evaluator reference](../../../../plugins/jev-evaluator/README.md) defines evaluation behavior; read without changing it by default.

Expected result: One click sends exactly one selected prompt to the correct conversation.
With auto-send enabled, an eligible Jev-approved prompt runs without a click, within the bounded chain.

## Verify

- Run the new plugin's declared format, typecheck, lint, and focused test scripts; all pass.
- Test heading variants, multiple prompts, multiline Vietnamese, incomplete streams, and non-prompt code blocks.
- Test rapid double-clicks, concurrent clients, manual/auto races, reconnects, and ambiguous network results; no duplicate dispatch occurs.
- Test conversation switches, newer user turns, busy agents, and preserved composer drafts.
- Test default-off, persisted settings, no historical replay, rejection, missing Jev, timeout, invalid answers, and missing prerequisites.
- Test toggle-off during evaluation, user interruption, and the three-turn limit; no later automatic send occurs.
- Test completed work and requests requiring new authority; automation stops instead of granting permission.
- After installation, verify plugin status is running without load errors. Record installed host version and supported integration.
- In the real UI, verify desktop and narrow layouts, light/dark themes, keyboard focus, copy, Send, status, and Retry.
- Verify one click creates exactly one actual user turn in the source conversation with the expected text.
- With synthetic authorized input, verify live Jev approval causes one real send; rejection and toggle-off cause none.
- Disable or reload the plugin; owned controls and subscriptions clean up without changing existing chat content.
- Record failed or unavailable checks explicitly. Design images and running status alone do not prove runtime behavior.
