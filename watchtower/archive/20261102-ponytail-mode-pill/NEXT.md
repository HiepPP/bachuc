# NEXT

## Current Active Plan

- Title: Ponytail mode pill and global Claude install
- Slug: 20261102-ponytail-mode-pill
- Status: ARCHIVED
- Updated: 2026-11-02

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                   | Group | Status | Spec                                                                                                             | Deps               | Context                                        | Notes                                                             |
| ----- | -------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | ----------------------------------------------------------------- |
| 1     | TASK-325 Ponytail server plugin        | A     | DONE   | [watchtower/tasks/TASK-325-ponytail-server-plugin.md](watchtower/tasks/TASK-325-ponytail-server-plugin.md)       | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Mode store, RPCs, daemon hook, bridge hook, installer.            |
| 2     | TASK-326 GPUI Ponytail pill            | B     | DONE   | [watchtower/tasks/TASK-326-gpui-ponytail-pill.md](watchtower/tasks/TASK-326-gpui-ponytail-pill.md)               | TASK-325           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Starts after TASK-325; its registry test needs the plugin folder. |
| 3     | TASK-327 Global install and live check | C     | DONE   | [watchtower/tasks/TASK-327-global-install-live-check.md](watchtower/tasks/TASK-327-global-install-live-check.md) | TASK-325, TASK-326 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Writes `~/.claude` and both Paseo homes; no repo source.          |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- A new Claude agent on live, with the pill at Full, gets the Ponytail ruleset on its first turn -> `$PASEO_HOME/plugin-data/ponytail/agents/<id>/last-hook.json` shows `mode: full` and `kind: rules`.
- A plain `claude -p` run outside Paseo gets the native Ponytail SessionStart context through the bridge -> the bridge prints the ruleset for a SessionStart input with no `PASEO_AGENT_ID`.
- `claude plugin list` shows `ponytail@ponytail` installed and disabled.

## Review Focus

- The agent types `/ponytail off` in the composer while the pill shows Full -> the daemon saves `off` and the next turn gets no rules; owner TASK-325.
- The Ponytail install path is missing or moved after an update -> turns go out with no Ponytail context and never fail; owner TASK-325.
- The daemon or plugin restarts in the middle of a thread -> the next turn gets the full ruleset again, not only the reminder; owner TASK-325.
- A thread compacts after many turns -> the full ruleset comes back within 10 turns; owner TASK-325.
- A Paseo Claude agent fires SessionStart through the global bridge -> the bridge prints nothing, so the native default does not override the pill; owner TASK-325.

## Handoff

- Next action: Hiep installs the plugin on release (Q-201), then reviews and merges `feat/ponytail-mode-pill`.

## Archive

- Archived: 2026-11-02 -> watchtower/archive/20261102-ponytail-mode-pill/
