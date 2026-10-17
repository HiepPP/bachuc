# TASK-192 Outcome

## Status

DONE

## Changed

- `CLAUDE.md`: the "Code a plugin" bullet names the GPUI client crate, says the React client is frozen, says `plugin reload` reloads only the server part, and links the new section. The `docs/plugins.md` table row mentions GPUI clients.
- `docs/plugins.md`: the intro sentence covers both clients. New section "GPUI native clients" before "Lifecycle hooks".
- `packages/desktop-gpui/docs/parity-audit.md`: date 2026-10-17; "How to read" note; Composer rows for the lightning toggle and diff-count pill now UNVERIFIED; new rows for the track strip and plugin pills; new "New workspace and plugins" table; ranked mismatch 3 rewritten; "Plugin surfaces" and "New workspace screen" removed from out-of-scope.

## Contract

Docs only. No code, no Produces line.

## Verified

- `npm run format:check:files -- CLAUDE.md docs/plugins.md packages/desktop-gpui/docs/parity-audit.md` -> exit 0.
- `rg -n "packages/desktop-gpui/plugins" CLAUDE.md docs/plugins.md` -> one match in each file.
- `rg -c "^## GPUI native clients" docs/plugins.md` -> 1.
- `rg -n "in every connected app" docs/plugins.md` -> no match.
- Out-of-scope rows rg -> no match.
- `git diff --numstat -- CLAUDE.md` -> 2 added, 2 removed.
- Links: ADR-0045 file exists; `#gpui-native-clients` anchor matches the heading.

## Anti-goal

CLAUDE.md added minus removed lines: 0 (limit 4). Source: `git diff --numstat -- CLAUDE.md`, at completion.

## Lessons

- Every UI check of TASK-168 to TASK-191 is DEFERRED (UI checker), so the audit marks the new rows UNVERIFIED. The UI checker must replace them with measured values.
- Lightning toggle and diff-count pill status come from the code (`fast_mode` in `agent_controls/features.rs`, `diff_stat_pill.rs`); no outcome states them. @file mentions have no code and stay MISMATCH (Q-080).
