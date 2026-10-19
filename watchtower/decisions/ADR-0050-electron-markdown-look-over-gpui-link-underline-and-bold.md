# ADR-0050 Electron markdown look over the GPUI link underline and 700 bold

Status: accepted
Date: 2026-10-19
Supersedes: -
Scope: `packages/desktop-gpui/vendor/gpui-base/src/text/**`, `packages/desktop-gpui/crates/timeline/src/lib.rs`, `packages/desktop-gpui/crates/timeline/src/links.rs`

## Context

- Electron draws assistant links with no underline (`link`, `packages/app/src/styles/markdown-styles.ts:154-161`) and bold at weight 500 (`strong`, `markdown-styles.ts:138-141`).
- Electron draws inline code as a `surface2` pill, also when the code is a file link (`packages/app/src/assistant-file-links/link.tsx:104-194`).
- The vendored GPUI text view underlines every link (`packages/desktop-gpui/vendor/gpui-base/src/text/node.rs:2841-2847`), draws bold at 700 (`node.rs:2393-2395`), and loses the code pill inside a link.
- The owner compared both clients on one thread on 2026-10-19 and wants the Electron look.

## Decision

- Patch the vendored text view so all GPUI assistant markdown matches Electron: no link underline, the code pill kept inside a link, and bold at weight 500.
- The owner chose this in the planning interview on 2026-10-19.

## Consequences

- Every GPUI link loses its underline; the link color alone marks it.
- The patch adds to `vendor/gpui-base`, so `BACHUC-PATCH.md` must record it and a gpui-kit upgrade must re-apply it ([ADR-0044](watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md)).

## Options Considered

- The Electron look in step notes only: rejected. Replies and notes in one thread would then use two link and bold styles.
- Keep the GPUI link underline and fix only the bold and the code pill: rejected. The owner wants full parity with Electron.

## Revisit If

- The owner reports that GPUI links are hard to see without an underline.
- Electron changes its `link` or `strong` style.
