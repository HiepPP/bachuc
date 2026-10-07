# Decisions

Durable decisions for this repo. Read this index before `new`, `implement`, and `archive`. Skip rows that are not `accepted`. Open an ADR only when its Scope matches the current work.

## Index

| ID       | Date       | Title                                                | Status   | Scope                                                                                                                                              | File                                                                                  |
| -------- | ---------- | ---------------------------------------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| ADR-0001 | 2026-09-30 | Active host over sidebar host filters                | accepted | packages/app/src/hooks/use-sidebar-workspaces-list.ts, packages/app/src/stores/sidebar-view-store.ts, packages/app/src/components/left-sidebar.tsx | [ADR-0001](watchtower/decisions/ADR-0001-active-host-over-host-filters.md)            |
| ADR-0002 | 2026-10-06 | Networked HTML replies over offline-only frames      | accepted | `hiep-plugins/plugins/html-reply/**`, `packages/app/src/plugins/react-native/html-frame*`, `packages/plugin/src/client/ui.ts`                      | [ADR-0002](watchtower/decisions/ADR-0002-networked-html-replies-over-offline-only.md) |
| ADR-0003 | 2026-10-07 | Chip-only attachment sources for chip press handlers | proposed | `packages/plugin/src/contracts.ts`, `packages/app/src/plugins/evaluate.ts`, `packages/app/src/plugins/attachments/**`                              | [ADR-0003](watchtower/decisions/ADR-0003-chip-only-attachment-sources.md)             |
