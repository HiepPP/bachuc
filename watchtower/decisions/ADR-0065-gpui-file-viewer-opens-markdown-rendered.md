# ADR-0065 GPUI file viewer opens markdown rendered over source first

Status: accepted
Date: 2026-10-22
Supersedes: -
Scope: `packages/desktop-gpui/crates/explorer/src/file_view.rs`

## Context

- The Electron file pane shows the markdown preview unless the file opens at a line or the mode is `source` (`FilePreviewBody` in [packages/app/src/file-pane/pane.tsx](packages/app/src/file-pane/pane.tsx)).
- The GPUI file viewer showed only numbered source lines.

## Decision

- A `.md` or `.markdown` file opens Rendered, except when it opens at a line, which opens Source. A toolbar toggle switches per view.
- Hiep chose this on 2026-10-22 in the watchtower interview for TASK-289.

## Consequences

- GPUI matches Electron for markdown files.
- A reader who wants source presses "Show source" each time; the mode is per view and not saved.

## Options Considered

- Always open Source and offer a toggle: rejected. It differs from Electron.

## Revisit If

- Hiep wants the last chosen mode remembered across views.
