# TASK-310 outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/src/composer.rs`: new `ComposerEvent::Delivered { agent_id: String }`; one emit in the success branch of the spawned task in `finish_submit`.

## Contract

- Produces `ComposerEvent::Delivered { agent_id: String }`, emitted after a send or queue reply when `send_error_text(&result)` is `None`.
- `Sent` and its timing are unchanged. The draft path returns before the spawn, so a draft never emits `Delivered`. A failed send keeps its error and text restore.
- `bachuc-app` matches `ComposerEvent` with `_ => {}` or `==`, so no match needed an edit.

## Verified

- `./cargo.sh test -p bachuc-composer` -> 140 + 5 passed, 0 failed (group-level run, the only run).
- `rg -n "ComposerEvent::Delivered" composer.rs` -> one emit (line 1790), inside the success branch.
- `./cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `./cargo.sh check -p bachuc-app` -> Finished, no errors.
- Clippy `-p bachuc-composer -p bachuc-app --no-deps -- -D warnings`: UNSATISFIABLE as written. The baseline already fails with 10 errors in other composer files. The error list before and after the change is identical, and none points at `composer.rs`.
- Anti-goal checkpoints (`./cargo.sh clippy ... -D warnings`):
  - before: 10 errors (`plugin_slots.rs:161`, `plugin_slots.rs:340`, `subagents_track.rs:63/195/197/198`, `input.rs:20`, `agent_controls/mod.rs:317/318`, `attachment_tray.rs:88`), saved in `/tmp/t310-clippy-before.txt`.
  - after: the same 10 errors, no new one, `/tmp/t310-clippy-after.txt`. Result: no new warning (tripwire not crossed); the 0-warning baseline never held.
  - `bachuc-app` clippy did not run, because `bachuc-composer` fails first; `cargo check -p bachuc-app` passes.
- First test attempt hit a compile error in `bachuc-settings` (another builder's crate, `after_send.rs` in progress). It cleared on retry.

## Anti-goal

PASS on "no new warnings". The "0 warnings" wording is wrong: replace it with "clippy output unchanged from the 10 pre-existing composer errors".

## Lessons

- `bachuc-composer` fails `clippy -D warnings --no-deps` on 10 old lines (Rust 1.97), so a Verify line cannot demand 0 warnings; diff the error list instead.
- Clippy stops at the first failing crate, so `bachuc-app` never gets linted in the same command.
