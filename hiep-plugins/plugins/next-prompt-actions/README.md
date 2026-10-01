# Next prompt actions

Send suggestions under `What Next`, `What's Next`, or `Next Steps` directly from the reply, on
every Paseo client. Requires the Paseo fork `>=0.10.2-beta.900`; the stable 0.10.1 host refuses it.

- The reply keeps its native Markdown up to the What Next heading. The rest becomes one panel
  with the heading, its intro text, and the suggestions. The raw fences are not shown.
- Each suggestion has **Edit**, which fills this conversation's composer, and **Send**.
  **New thread** starts the prompt in a separate conversation. A `thread: new` suggestion shows
  **Start in new thread** instead of Send.
- Structured `next-prompts` v1 blocks with several suggestions use radio and checkbox rows that
  follow `exclusiveGroups` and `allowedCombinations`, with **Edit selected** and **Send selected**.
  No alternatives or default selections are invented.
- Explicit Git suggestions replace Send with one action: Commit sends `/commit --no-push`,
  Commit & Push sends `/commit` only when push is explicitly requested, and Push sends the
  original push-only suggestion. Git actions have no New thread button, and Jev never auto-runs
  them.
- An optional `why:` line shows as the suggestion's reason and is never sent.
- Only the latest reply offers buttons. Earlier panels stay readable without them.
- The panel is a timeline plugin row. Recap sections and other text before the What Next
  heading keep their native rendering.

## Structured suggestions

Use one JSON fence with the language `next-prompts`. The first suggestion is the recommended next step.

````markdown
## What Next

```next-prompts
{
  "version": 1,
  "prompts": [
    { "id": "implement", "prompt": "Implement the settings layout.", "why": "Apply the agreed design." },
    { "id": "review", "prompt": "Review the settings layout. Do not change code." },
    { "id": "risks", "prompt": "List remaining risks." }
  ],
  "exclusiveGroups": [["implement", "review"]],
  "allowedCombinations": [["implement", "risks"], ["review", "risks"]]
}
```
````

- `version` must be `1`. Each prompt has a unique lowercase ID, exact `prompt` text, an optional `why`, and an optional
  `"thread": "new"`. The block may set `"goal": "done"`.
- `exclusiveGroups` declare disjoint radio groups. Each group permits at most one selection, not a required selection.
- Other suggestions use checkboxes. Nothing is selected automatically. Clear selection resets all controls.
- `allowedCombinations` lists exact permitted sets of IDs. Subsets, supersets, and transitive combinations are not inferred.
- Both relationship arrays may be omitted; the default is no permitted bulk sends. Single sends remain available.
- Checkboxes are grouped by declared combinations ([design](../../docs/designs/next-prompts-combos-2026-09-27/combos.html)):
  Send together holds suggestions linked by combinations (numbered sets when there are several), Follow-up holds those
  whose combinations include an exclusive choice, and Send alone holds suggestions in no combination.
- A suggestion that no declared combination can join with the current selection is disabled and labelled
  "Not with selection" or "Send alone". A selection that fits a combination only partially keeps Send disabled.
- The server revalidates every selection from the current reply. Separate fences cannot be combined.
- Selected prompts are sent in authored order as one numbered message. Only `prompt` text is sent, never IDs, relationships, or reasons.
- Busy, stale, duplicate, uncertain-send, and individual Git-action guards still apply. Blocks containing Git actions use individual controls.
- Relationships declare intent; they do not prove semantic compatibility, grant permissions, or authorize parallel execution.

Unknown fields, versions, invalid references, overlapping exclusive groups, duplicate IDs or sets, and contradictory declarations reject the entire block.
Malformed, partial, or unsupported blocks remain visible as plain code without plugin controls.
Limits: 64 KiB per JSON block, 1–20 prompts, 16,000 characters per prompt, 2,000 per reason,
64 characters per ID (`[a-z][a-z0-9-]*`), 20 exclusive groups, and 64 allowed combinations.
Each group or combination contains 2–20 distinct IDs. Legacy blocks retain their existing parser limits.

## Goal and new-thread suggestions

- `"goal": "done"` at the top of a `next-prompts` block says the reply finished its task. The panel shows a
  Task done chip in the Recap header, or next to the What Next title when no Recap is folded.
- `"thread": "new"` on a prompt, or a `thread: new` line in a legacy fence, marks work unrelated to the current task.
  These prompts appear in a separate Other work section with one Start in new thread button and no Edit or Send.
- Start in new thread creates a separate conversation with the source conversation's directory, provider, model,
  mode, and thinking option; the prompt is its first message. It works while this conversation is busy.
  The source conversation, its composer draft, and Board events are unchanged.
- A start is reserved before dispatch and uses the suggestion key as the idempotency key; an uncertain result shows
  Check threads and is never retried.
- New-thread prompts cannot appear in `exclusiveGroups` or `allowedCombinations`; such a block is rejected.
  Send rejects them server-side unless one is sent alone by a manual Cmd-click. Jev auto-run ignores them.

## Compatibility

Uses the plugin timeline API with `source` kept, `setComposerText`, and cross-plugin
`openSurface`, all added in the Paseo fork `0.10.2-beta.900`. No DOM code. The panel needs the
agent's workspace, which the daemon resolves through `prompts.scope`.

## Back to Board after send

Settings → Next prompt actions → Back to Board after send. Host setting; defaults OFF.
When ON, a manual Send opens the Board surface of the `board` plugin at once, and the send
finishes in the background. Missing or disabled Board registration adds a warning to the panel.

## Jev auto-run

The per-conversation setting defaults OFF. Turning it ON applies only to new turns, never old suggestions.
Jev evaluates the original user goal, user-message context, and the proposed continuation.
Exactly one suitable suggestion with a positive `send` decision probability above 0.5 can be sent.
This judgment is not proof of correctness and does not change agent permissions or grant new authority.
Missing context, multiple suggestions, failed evaluations, and rejected decisions require manual Send.
Each chain is limited to three evaluations/sends. User interruption, permission requests, or OFF cancel pending automation.
Use Command Center → Toggle Jev next-prompt auto-run to enable or disable automation for the current conversation.
Prompt blocks contain Send and action status only; they do not contain the automation setting.

Requires the existing directory-installed `jev-evaluator` plugin and its Gateway configuration.
Evaluation runs in a Node child with the existing evaluator contract; no new chat provider is created.
Credentials remain server-side. Each evaluation uses API quota; no automatic API retries occur.
Local settings, deduplication hashes, and the chain's user goal live under `$PASEO_HOME/plugin-data/next-prompt-actions/`.
Never add those files to Git. A send with an uncertain acknowledgement displays Check chat and cannot retry automatically.

## Install and verify

```sh
npm install
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id next-prompt-actions
paseo plugin ls next-prompt-actions --json
```

For updates, run the checks then `paseo plugin reload next-prompt-actions`.
Check actual Send, target conversation, preserved draft, dark/light layout, and toggle behavior after reload.
Disable removes owned DOM controls, styles, listeners, and cancels pending evaluations.

Long threads need only a contiguous timeline tail containing the latest user message and reply.
Older omitted turns do not hide Send; gaps or a missing latest-turn boundary still fail closed.

Missing evaluator files or Gateway SDK add a specific warning. Auto-run cannot be enabled until restored; manual Send remains available.

Board notifications carry the sending host ID. Update Board on that host for Back to Board support; older listeners safely ignore the versioned events.
