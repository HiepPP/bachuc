# Plan Context

## Shared Context

- The work is in the GPUI client only: [packages/desktop-gpui/crates/explorer/src](packages/desktop-gpui/crates/explorer/src). No protocol, server, or Electron change.
- Run cargo only through `./cargo.sh` in `packages/desktop-gpui`. After tests, run `./cargo.sh clean --profile dev` when no other cargo process runs ([CLAUDE.md](CLAUDE.md)).
- GPUI view crates have no headless test context. Put pure rules in a model module with tests in [packages/desktop-gpui/crates/explorer/tests/models.rs](packages/desktop-gpui/crates/explorer/tests/models.rs) ([docs/gpui.md](docs/gpui.md)).
- Render markdown with `TextView::markdown` from gpui-kit, as [packages/desktop-gpui/crates/explorer/src/pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs) does. Add no third-party crate.
- A markdown file is `.md` or `.markdown`, any case, as `isRenderedMarkdownFile` in [packages/app/src/components/file-pane-render-mode.ts](packages/app/src/components/file-pane-render-mode.ts).
- Screen checks use `cua_repl` only and never take the cursor or the front ([docs/gpui.md](docs/gpui.md#screen-checks)).

## Decisions

- Follows [ADR-0061](watchtower/decisions/ADR-0061-gpui-rich-markdown-diff-from-working-file-over-protocol-full-text.md): the rich diff reads the working file and reverses the patch; no protocol change.
- Rich diff is only in the one-file uncommitted diff (`Source::File`). The all-files view and commit diffs get no markdown toggle. Owner chose this on 2026-10-22.
- The file viewer opens a markdown file Rendered, except when it opens at a line. Owner chose this on 2026-10-22.

## Open Decisions

- None.

## References

- [packages/desktop-gpui/crates/explorer/src/diff_view.rs](packages/desktop-gpui/crates/explorer/src/diff_view.rs)
- [packages/desktop-gpui/crates/explorer/src/file_view.rs](packages/desktop-gpui/crates/explorer/src/file_view.rs)
- [packages/desktop-gpui/crates/explorer/src/diff_prefs.rs](packages/desktop-gpui/crates/explorer/src/diff_prefs.rs)
- [packages/app/src/file-pane/pane.tsx](packages/app/src/file-pane/pane.tsx)
