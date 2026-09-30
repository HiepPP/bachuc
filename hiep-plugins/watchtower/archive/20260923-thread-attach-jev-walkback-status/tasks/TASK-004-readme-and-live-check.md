# TASK-004 README and live check

Group: C (standalone; writes README.md only)
Class: docs

## Brief

Goal: Document the opt-in Jev mode and prove it works in the real app once.

Change: README describes only the last reply -> README also describes the Jev setting, the new format, cost, and privacy.

How:

- Add a `## Jev mode (opt-in)` section. Explain the setting, the status line, the 1 to 3 reply walk-back, and that the first search shows the plain snapshot.
- State what is sent to Vercel: the thread title and up to 3 assistant replies, each cut to 4,000 characters. State that it needs the `jev-evaluator` plugin and a Gateway key.
- State the fallback: Jev off, missing, failing, or slow means the current format.
- Reload the plugin and run one live check. This makes a billable Jev call.

Files:

- [plugins/thread-context-attach/README.md](plugins/thread-context-attach/README.md) (new section and updated format example)

Expected result:

- The README matches shipped behavior.
- One live check is recorded in the outcome sidecar.

## Verify

- `cd plugins/thread-context-attach && npx oxfmt --check README.md` -> exit 0.
- `paseo plugin reload thread-context-attach && paseo plugin ls thread-context-attach --json` -> plugin is enabled with no load error.
- Manual, needs the Paseo app and a billable Jev call: turn the setting on, open + -> Thread twice, and attach a thread. The text has a `Status:` line. `paseo plugin logs thread-context-attach` shows one Jev line with usage and no reply text.
