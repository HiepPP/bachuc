# Assistant cite

Client-only Paseo plugin. Select text in an assistant reply and click **Cite**: a quote chip joins
the agent's composer, and the draft text stays. Write an optional comment on the chip. A press on
the chip jumps back to the quoted passage.

## What the agent gets

```text
Quoted from your earlier reply:

> the selected text, as Markdown

Comment: your comment, when you wrote one
```

The blockquote under the lead line marks the quote as reference text, not as new instructions.
The host appends the `Comment:` line.

## Behavior

- The Cite action shows on web and desktop only. Native apps have no selection toolbar.
- A selection across several messages cites them together and jumps back to the first one.
- Quotes are limited to 4000 characters. A longer selection adds no chip and logs a warning,
  because plugins have no toast API yet.
- Citing the same passage again replaces its chip and keeps your comment.
- The jump back highlights the first line of the quote when it appears once in the message. On
  native, it opens the agent without scrolling.
- The chip stores its source in the item, so it still opens after a reload or on another client
  with this plugin installed.

## Host version

Needs the fork host from `t3code-port` (`requirements.paseo >=0.10.2-beta.900`) with these APIs:
`addAssistantSelectionAction`, `addComposerAttachment` with `commentable`, `revealTimelinePassage`,
and attachment sources with `onOpen` and no `search`.

## Develop

```sh
npm install
npm test
npm run typecheck
npm run lint
npm run format
```

Install on live from the Paseo repo root:

```sh
npm run cli -- plugin install "$PWD/hiep-plugins/plugins/assistant-cite" --id assistant-cite
npm run cli -- plugin reload assistant-cite
```
