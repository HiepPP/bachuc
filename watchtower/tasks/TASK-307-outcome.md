# TASK-307 Outcome

## Status

DONE. The two screen checks are `DEFERRED (UI checker)`.

## Changed

- `packages/desktop-gpui/crates/timeline/src/tools.rs`: `HookLine`, `hook_lines`, the `hook` name shows "Hook" with the Zap icon, and `has_details` counts hooks.
- `packages/desktop-gpui/crates/timeline/src/tool_view.rs`: a Zap badge with the hook count in the tool row header (error color when any hook has status `error`), and a Hooks section after the detail sections in the open row.
- `packages/desktop-gpui/crates/timeline/tests/tools.rs`: the four red tests of the spec, unchanged, and the import line extended with `HookLine` and `hook_lines`.

## Contract

- Consumes `hooks?: ToolCallHook[]` and the `hook` tool call item (`callId: "hook:<id>"`) from TASK-305, read as `hooks[].name/status/exitCode/output` and `detail.label`.
- Safe default: a `hook` row (name `hook`) shows no badge and no Hooks section of its own, because it is itself one hook. Its output already shows as the plain detail text.
- A tool call with no `hooks` field draws as before. No row is added, so the grouper is unchanged.
- Impact (GitNexus): `render_badge` LOW (2 callers); `presentation` LOW; `has_details` UNKNOWN, so confirmed by `rg`: only the test file calls it. No HIGH or CRITICAL.

## Verified

- Red: the test file failed to compile first (`unresolved imports HookLine, hook_lines`), before any other edit.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> all suites pass, including the four new tests and `incremental_rows_match_full_regroup`.
- `./cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- `./cargo.sh clippy -p bachuc-timeline --no-deps -- -D warnings` -> 3 errors, all on old lines (`plugin_view.rs:185`, `lib.rs:376`, `lib.rs:3466`; blame of `lib.rs:3466` is 2026-10-16). None in files this TASK touched. Known Failure, not fixed.
- Read check of `tool_view.rs`: the badge is built in `render_badge` and sits in the header row before the open-file button; `render_hooks` sits inside the expanded `badge-detail` body.
- Screen check, Bash row with Zap badge and listed hooks: DEFERRED (UI checker).
- Screen check, `SessionStart` "Hook" row with Zap icon: DEFERRED (UI checker).
- Cleanup `./cargo.sh clean --profile dev`: not run, because another cargo process (group C) was running.

## Anti-goal

Tripwire: `incremental_rows_match_full_regroup` in `bachuc-timeline`. Source: `./cargo.sh test -p bachuc-timeline`.

- Before changes: ok (2026-10-27 01:26).
- At completion: ok (2026-10-27 01:31). No flush or grouping code changed.

## Lessons

- `has_details` has no caller in the view: `render_badge` decides `can_open` from `sections`, so a new openable source must be added in both places.
- BSD `sed -i` needs a backup suffix argument; use Edit for one-line changes.

## UI Checks

Run on 2026-10-27 with the check bundle on live (6768), fixture agent "wt ui fixture" (Claude haiku, owner hooks), 900x800 window. Screens: `/tmp/wt-ui-check/`.

- C1: PASS. The Bash tool row shows "Shell ls" with a Zap badge and `6`. Opening it shows a Hooks section with 3 `PreToolUse:Bash` and 3 `PostToolUse:Bash` lines, each with the name, `success`, `exit 0`, and its output when set (`/tmp/wt-ui-check/c1-shell-open.png`, `c1-hooks-list.png`).
- C2: PASS. `SessionStart:startup` (3), `UserPromptSubmit` (6), and `Stop` (5 in step 2) show as compact rows with the Zap icon, "Hook", and the hook name (`/tmp/wt-ui-check/c2-stop-rows.png`).
- Note: the step summaries count hook rows as "other tools" (step 1: "ran 1 command, used 9 other tools"; step 2: "used 5 other tools", which is five Stop hooks). This may not be what the plan wants.
- Note: `cua_repl` screenshots of a background GPUI window lag one action behind. The AX tree was current. A frame was forced with the AX "zoom the window" action, done twice so the window returned to 900x800.
