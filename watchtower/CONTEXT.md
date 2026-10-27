# Plan Context

## Shared Context

- All code is in the GPUI client under [packages/desktop-gpui](packages/desktop-gpui). Run cargo only through `./cargo.sh`. Run `./cargo.sh clean --profile dev` after builds, unless another cargo runs.
- The GPUI client shows one host at a time ([ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md)). The queue covers only the active host `Shell::client`.
- Unread means the daemon attention flag: `Client::agent_attention` returns `(requires_attention, reason)`. `Shell::sync_viewed_agent` in [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) clears it when a thread is viewed.
- The queue holds agents with `requires_attention` true and reason `Finished` or `Error` that are also a Board run with a status other than `running` (ADR-0072). It leaves out `Permission`, subagents (`Client::agent_parent_id` is `Some`), and the viewed agent. The oldest `attentionTimestamp` comes first.
- The Board is the plugin `board` with the page surface `board`, in [packages/desktop-gpui/plugins/board/src/lib.rs](packages/desktop-gpui/plugins/board/src/lib.rs).

## Decisions

- [ADR-0072](watchtower/decisions/ADR-0072-board-cards-only-unread-queue.md), superseding ADR-0070: the host switcher, the unread Board-card FIFO queue, and the Mod+J keys.
- [ADR-0071](watchtower/decisions/ADR-0071-after-send-setting-over-fixed-jump.md): the After send setting with Stay as default.

## Open Decisions

- None.

## References

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs)
- [docs/gpui.md](docs/gpui.md)
