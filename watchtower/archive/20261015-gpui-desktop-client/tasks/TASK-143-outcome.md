# TASK-143 Outcome

## Status

DONE

## Changed

- `watchtower/research/RESEARCH-001-plugin-ui-host.md` (new)
- `watchtower/RESEARCH.md` (new index, one row)
- `watchtower/decisions/ADR-0038-native-data-kinds-and-webview-panes-over-all-webview-or-rewrite.md` (new, proposed)
- `watchtower/DECISIONS.md` (one proposed row)

## Contract

Recommended: native GPUI views for the 10 data kinds, a plugin pane (a `TabContent` variant with one live WebView outside scroll lists) for pages and panels, PNG snapshots for timeline renderers. ADR-0038, `proposed`; no MEMORY.md `## Avoid` item.

## Verified

- All 16 `index.client.tsx` entries contribute UI; 14 contribution kinds counted by `client.add*` calls.
- `rg -n "^\| " watchtower/research/RESEARCH-*-plugin-ui-host.md` -> table rows for each kind, 3 tables.
- `rg -n "proposed" watchtower/DECISIONS.md` -> one new row (ADR-0038).
- No probe was built; the WebView cost cites TASK-137 and TASK-141 outcomes.

## Anti-goal

`git status --short -- . ':!watchtower'` at 01:13:53 on 2026-10-16 -> no output from this TASK.

## Lessons

- Most plugin UI is data (pills, commands, settings), so a WebView per surface wastes 55 to 83 MB on small items.
- The visible-WebView CPU cost is unmeasured; only the hidden idle case has a number.
