# Next prompt actions

Send suggestions under `What Next`, `What's Next`, or `Next Steps` directly from the reply, on
every Paseo client. Requires the Paseo fork `>=0.10.2-beta.900`; the stock 0.10.1 host refuses it.

- The panel follows the stable layout: a compact Recap, next-step heading, Markdown reasons,
  bordered suggestion rows, and neutral Send controls. A complete preceding Recap folds into
  the panel (see [Recap](#recap)); unrecognized prose stays in the native reply.
- Each suggestion has **Edit**, which fills this conversation's composer, and **Send**.
  Hover the action and hold Command (Control off macOS) to swap Send with New thread.
  Both labels reserve their width, so swapping never moves adjacent controls. Native clients
  show a visible New thread action. A `thread: new` suggestion offers Start in new thread only;
  the modifier swaps it back to Send in the current conversation.
- Structured `next-prompts` v1 blocks with several suggestions use radio and checkbox rows that
  follow `exclusiveGroups` and `allowedCombinations`, with **Edit selected** and **Send selected**.
  No alternatives or default selections are invented.
- Explicit Git suggestions replace Send with one action: Commit sends `/commit --no-push`,
  Commit & Push sends `/commit` only when push is explicitly requested, and Push sends the
  original push-only suggestion. Git actions have no New thread button, and Jev never auto-runs
  them.
- An optional `why:` line shows as the suggestion's reason and is never sent.
- An optional `suggestion: true` or `suggestion: false` line marks whether the agent recommends
  the prompt (see [Suggested badge](#suggested-badge)).
- Only the latest reply offers buttons. Earlier panels stay readable without them.
- The panel never resumes a closed thread. Reading a timeline resumes its runtime, so a thread
  whose runtime was closed (by `idle-runtime-closer`, or by Remove in `board`) is served from the
  last read of its latest turn. Its buttons stay enabled; Send resumes the thread. A closed thread
  the daemon plugin has not read since it loaded is read once.
- Layout ports the original `client/web.ts` rules: two columns, non-shrinking actions,
  a separate selection footer, and stacked rows only below the 600px available-width breakpoint.
- The panel uses host-owned Markdown and Button components through the plugin UI API; no DOM patching.

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

- `version` must be `1`. Each prompt has a unique lowercase ID, exact `prompt` text, an optional `why`, an optional
  `"thread": "new"`, and an optional `"suggestion": true` or `false`. The block may set `"goal": "done"`.
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

## Suggested badge

The agent marks each prompt it recommends doing next. Several prompts, including new-thread prompts, may be suggested.

- Structured fence: `"suggestion": true` or `"suggestion": false` on a prompt. Absent means `false`.
  A value that is not a boolean counts as `false`; it never rejects the block.
- Legacy fence: a `suggestion: true` or `suggestion: false` line under a prompt, matched without regard to case like `thread: new`.
  The first such line of a prompt decides. These lines are never part of the prompt text. Other values, such as `suggestion: maybe`, stay in the prompt text.
- A suggested prompt shows a `Suggested` badge at the top right of its card: the direct Edit and Send card, the checkbox and radio row,
  the read-only card of an earlier reply, and the Other work card. The badge has its own row above the text, reason, buttons, and choice state.
  It uses the fixed fill `#0f7b5f` with white text in both themes. It is plain text that assistive tech reads. A checkbox or radio row is
  described by it: `aria-describedby` on web, an accessibility hint on native clients.
- The panel reads the flag from the reply's own fence, so earlier replies keep their badge; the server sets `suggestion` on a candidate only when true.
- The flag is display only. It never changes the sent prompt text, selection rules, relationship validation, or Git-action detection. Cards without it render as before.

## Recap

A Recap directly before What Next or Next Steps folds into the panel. Three field sets are recognised.
Any other field set or order, a missing label, an extra unlabeled block, or a quoted or fenced Recap keeps its native rendering.
A Recap with no suggestion panel after it is not folded.

- Compact, the shape the global agent rules ask for: one bullet list whose first item is
  `<branch> · <commit state>`, then `Did:`, then optional `Open:` and `Need from you:`, in that order.
  Vietnamese replies use `Đã làm:`, `Còn lại:`, and `Cần bạn:`. The panel shows each label as
  written. Did, Open, and Need from you may nest a list.
- Five fields, in this order: `Branch`, `Commit/push`, `Did`, `Not yet`, `Need from you`.
  `Commit/push` is `no`, or `yes` followed by detail such as `yes, committed abc1234`.
  `Did`, `Not yet`, and `Need from you` may each hold a list. Write `nothing` for a field with nothing to report.
- Legacy three fields: `Branch`, `Did`, `Commit/push`. Its body stays one unlabeled Did block.

Accepted shapes of the five-field Recap:

- One top-level bullet list with exactly five items, one per label. Nested lists are allowed only inside Did, Not yet, and Need from you.
- Labeled lines, one field per line. A label line may be followed by a top-level bullet or numbered list that belongs to that field.
  The list may follow Did, Not yet, or Need from you only. A label with an empty value, such as `Not yet:`, is valid only when a list follows it.

```markdown
## Recap

Branch: main
Commit/push: no
Did: Rewrote the section. +12/-8.
Not yet:

- Item one.
- Item two.

Need from you: Open a new session and check the chip.
```

The header shows a Branch chip and a Commit chip. The Commit chip maps its value like this; its accessibility label keeps the full value (`Commit/push: <value>`).

| Value                                                                                             | Chip text                                                               | Icon   |
| ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ------ |
| `no`, or legacy `none`, with an optional trailing period                                          | No commit                                                               | commit |
| `yes` alone                                                                                       | Committed                                                               | check  |
| `yes` plus detail, such as `yes, committed abc1234`                                               | the detail without `yes` and its separator, such as `committed abc1234` | check  |
| `committed ...` or `pushed ...`                                                                   | the value as written                                                    | check  |
| `no changes`, `not committed`, `chưa commit`, `không có thay đổi`, with an optional note after it | the state with a capital first letter, without the note                 | commit |
| anything else                                                                                     | the value as written                                                    | commit |

The body of a five-field Recap is three sections: Did, Not yet, and Need from you. Each has its label above its value, with a divider between sections; lists, inline code, and links are kept.
A Not yet or Need from you section whose value is `nothing`, in any case and with an optional trailing period, is omitted. Did always shows, and a field that holds a list is never treated as nothing.

The parser reads the reply's Markdown text (`shared/section.ts`), not rendered nodes, so it works on every client.

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
