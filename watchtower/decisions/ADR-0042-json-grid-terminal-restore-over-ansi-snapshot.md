# ADR-0042 JSON grid terminal restore over the ANSI snapshot mode

Status: accepted
Date: 2026-10-16
Supersedes: -
Scope: `packages/desktop-gpui/crates/client/**`, `packages/desktop-gpui/crates/terminal/**`

## Context

- With `subscribe_terminal_request` restore mode `"full-snapshot"`, the live daemon sends opcode `0x05` ANSI text. Without a restore mode, it sends the `0x04` JSON cell grid with the same state ([TASK-136 outcome](watchtower/archive/20261015-gpui-desktop-client/tasks/TASK-136-outcome.md), Q-059).
- The frame format is in [packages/protocol/src/binary-frames/terminal.ts](packages/protocol/src/binary-frames/terminal.ts).

## Decision

- Subscribe terminals with no restore mode, so the first frame is the `0x04` JSON grid as `TerminalFrame::Snapshot`. A `0x05` frame plays as `Output`.
- The run took this default for Q-059; the owner confirmed it at archive on 2026-10-16.

## Consequences

- `bachuc-terminal` turns the grid into SGR text and resets the emulator on every `Snapshot`, including after each reconnect.
- The restored screen holds cell colors from the grid, not the original escape sequences.

## Options Considered

- Restore mode `"full-snapshot"` with `0x05` ANSI bytes: rejected. The client contract and the terminal view were built on the JSON grid, and the ANSI form needed a second restore path.

## Revisit If

- The daemon stops sending the `0x04` grid without a restore mode.
- A restored screen shows wrong colors or attributes that the ANSI form keeps.
