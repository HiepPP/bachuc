# ADR-0071 After send setting with Stay default over a fixed jump after send

Status: accepted
Date: 2026-10-27
Supersedes: -
Scope: `packages/desktop-gpui/crates/settings/**`, `packages/desktop-gpui/crates/composer/src/composer.rs`, `packages/desktop-gpui/crates/app/src/shell.rs`

## Context

- After Hiep sends a follow-up prompt, the thread stays open. Today Hiep goes back to the Board by hand.
- `ComposerEvent::Sent` fires before the daemon replies, so a jump on `Sent` could leave a thread whose send failed.
- Hiep chose in this session a setting with three modes over a fixed auto-jump or a post-send button.

## Decision

- A GPUI setting "After send" has the modes Stay, Back to Board, and Next unread. Stay is the default.
- The shell acts only on a new `ComposerEvent::Delivered`, which fires after a reply with no error.
- Next unread with an empty queue opens the Board.
- Hiep chose this on 2026-10-27.

## Consequences

- The composer emits one more event.
- The setting lives in `after-send.json` in the GPUI data folder, not on the daemon.

## Options Considered

- Always jump to the next unread thread: rejected. Hiep sometimes wants to watch the new turn start.
- A "Next" pill or toast after each send: rejected. It costs one more action per prompt.
- No change: rejected. It keeps the trip back to the Board after every prompt.

## Revisit If

- Hiep keeps one mode for a month and asks to drop the setting.
- The daemon send reply grows slow enough that the jump feels late, measured above 1 second.
