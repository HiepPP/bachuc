# Edit diffs

Shows each agent edit and write like Claude Code: a row with the file path and `+added −removed`
counts, and below it the diff with file line numbers and syntax colors. These rows stay outside
the Overview tool call groups, so they show even when the commands around them are collapsed.

- **Settings → Plugins → Edit diffs → Auto-expand edit diffs** opens every edit's diff in
  the chat. It is on by default. When off, press the row to open the diff.
- Removed lines show their old line number; added and unchanged lines show the new one.
- A write shows the whole written file as added lines, numbered from line 1. The provider
  sends no old content, so overwriting an existing file also shows only added lines.
- Only edit and write tool calls change. Read, shell, and other tool calls keep Paseo's rows.
- Each new agent's system prompt gets a rule: change files only with edit tools, never with shell
  scripts such as `python3 - <<EOF` or `sed -i`. A shell edit has no diff to show. Claude's
  bypass mode tells it to prefer shell edits, so the rule says it overrides that. Agents created
  before the plugin loaded keep their old prompt.

## Line numbers

| Provider data                           | Where the numbers come from                                                                   |
| --------------------------------------- | --------------------------------------------------------------------------------------------- |
| Unified diff with `@@ -a,b +c,d @@`     | The hunk headers.                                                                             |
| Codex unified diff with bare `@@`       | The `edit-diffs.locate` RPC finds each hunk in the file.                                      |
| Claude `old_string` / `new_string` only | The same RPC finds the new text, or the old text if the edit is still waiting for permission. |

The RPC reads the file on the daemon machine when the diff opens. Its numbers show where
the text is now, so a later change to the file can move or remove them. When the text is
not found, the gutter stays empty. If the edited text occurs more than once, the first match wins.

## Host requirements

Needs the fork's `tokenizeCode` and `SyntaxToken` exports from `@getpaseo/plugin/client/ui`
(added after `1.0.0-hiep` was first built). An app without them fails to load the plugin.

## Checks

```sh
npm run format
npm run typecheck
npm run lint
npm test
```
