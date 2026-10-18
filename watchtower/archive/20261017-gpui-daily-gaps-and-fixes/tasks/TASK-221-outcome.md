# TASK-221 outcome

## Status

DONE (code). The screen check is DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/timeline/src/lib.rs`: `footer_fork_boundary` reads `client.timeline_epoch(&self.agent_id)` instead of `let epoch = None;`, and passes it to `fork_boundary`. The stale comment about the missing epoch is gone. `fork_boundary` is unchanged.

## Contract

- Consumes: `Client::timeline_epoch(&self, agent_id: &str) -> Option<&str>` from TASK-220, unchanged.
- On a host with `agentForkContextCursor` and a loaded timeline page, the footer boundary has `cursor { epoch, seq: seqEnd }`, plus the message id when present. An assistant message without `messageId` now gets a fork button. With no cursor support and no message id, there is still no fork button.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> pass (lib 23 tests, with `fork_boundary_follows_the_turn_boundary_rule`). Log: `/tmp/t221-tl.txt`.
- `rg -n "let epoch = None"` -> no match. `rg -n "timeline_epoch"` -> one match (line 480).
- Anti-goal: `rg -n "footer_fork_boundary"` -> the definition plus the same 2 call sites, `render_footer` and the line 1663 caller, as before. The added work is one hash-map read per footer draw.
- `./cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> exit 0. Log: `/tmp/t221-app.txt`.
- GitNexus impact for `footer_fork_boundary` -> UNKNOWN (stale index, target not found). `rg` confirmed the 2 callers.
- UI check: fork from a completed turn of a disposable agent opens a draft with the chat history chip -> DEFERRED (UI checker).

## Lessons

- `timeline_epoch` borrows from the client read guard, so it fits the existing `let client = self.client.read(cx);` scope with no clone.

## UI Checks

Run 2026-10-18 by the UI checker (run 5). Bundle `Bachuc GPUI Check` built from HEAD `08064a6f3` (includes `626143dc7` and TASK-225 `699395706`), PID 34022, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). Frames were forced with the AX "zoom the window" action; keys went to the check app only. Scratch repo `/tmp/t202-repo` (branches `feature-x` and `main`, new commit `1f8a4a9` with a `web` service script) was the live workspace `wks_9cd2d5d43fcc7b08` with the disposable mock agent "r5 fork src" (`d8ef867`). Request counts come from the live `daemon.log` `ws_runtime_metrics` lines (30 s windows, all clients), parsed by `/tmp/r5-metrics.py`. After the run, PID 34022 exited, the agent and the workspace were archived, and the scratch terminal was killed.

- Fork from a completed turn -> a draft opens with the chat history chip: PASS. On "r5 fork src", the AX press on "Fork chat from here" opened the menu "Fork in a new tab" / "Fork in a new workspace". "Fork in a new tab" opened a "New agent" draft tab in the same workspace with the attachment "Chat history" and an empty composer. Nothing was sent. The daemon got one `agent.fork_context.request` (window 03:54:38). Evidence: `/tmp/r5-221-01-fork-draft.png`.
- Anti-goal: PASS. `rg -n "footer_fork_boundary"` -> line 470 (definition), 520, and 1663, the same 2 call sites as the outcome. `let epoch = None` -> no match; `timeline_epoch` -> 1 match.
- Cleanup: PID 34022 quit and exited. Hiep's app "Bachuc GPUI Test" (PID 9925, running at the start) was not running at the end; the checker sent it no input or signal, and no crash report exists. Live 6768 stays up.
