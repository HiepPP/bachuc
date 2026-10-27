# TASK-310 Composer Delivered event

Group: B (packages/desktop-gpui/crates/composer/src/composer.rs)
Class: code

## Brief

Goal: The composer tells its owner when the daemon accepted a prompt, so the shell can leave the thread only after a good send.

Change: `ComposerEvent::Sent` fires before the reply -> a new `ComposerEvent::Delivered { agent_id }` also fires after a reply with no error.

Boundaries:

- Keep `ComposerEvent::Sent` and its timing unchanged. Other code listens to it.
- Trap: `finish_submit` in `composer.rs` returns early for a draft. A draft submit must not emit `Delivered`; the draft has no agent yet.
- Emit `Delivered` for both `SendPlan::Send` and `SendPlan::Queue`, only when `send_error_text(&result)` is `None`.

How:

- In the spawned task of `finish_submit`, emit `ComposerEvent::Delivered { agent_id }` when the result has no error.

Files:

- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (new variant and emit)

Interfaces:

- Produces: `ComposerEvent::Delivered { agent_id: String }` for TASK-313

Expected result:

- WHEN a send gets a reply with no error THE composer SHALL emit `ComposerEvent::Delivered` with its agent id.
- IF the send fails THEN THE composer SHALL not emit `Delivered` and SHALL keep its current error and text restore.
- WHEN a draft submits THE composer SHALL not emit `Delivered`.

Anti-goal: the `bachuc-composer` and `bachuc-app` builds keep 0 warnings; tripwire; read from `./cargo.sh clippy -p bachuc-composer -p bachuc-app --no-deps -- -D warnings` before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-composer -p bachuc-app --no-deps -- -D warnings` -> no warnings. A non-exhaustive match in the app on `ComposerEvent` still compiles.
- `rg -n "ComposerEvent::Delivered" packages/desktop-gpui/crates/composer/src/composer.rs` -> one emit, inside the success branch of the send task.
