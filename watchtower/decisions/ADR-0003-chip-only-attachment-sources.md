# ADR-0003 Chip-only attachment sources for chip press handlers

Status: proposed
Date: 2026-10-07
Supersedes: -
Scope: `packages/plugin/src/contracts.ts`, `packages/app/src/plugins/evaluate.ts`, `packages/app/src/plugins/attachments/**`

## Context

- TASK-010 adds a chip `onOpen` handler on `addAttachmentSource`. TASK-007 (cite) registers a source with `onOpen` for its quote chips, which `addComposerAttachment` adds from code.
- `PluginAttachmentSourceContribution` required `pickerTitle`, `searchPlaceholder`, and `search`. Every source showed in the composer attachment menu.
- A chip is persisted in the draft, so its press handler cannot live on the chip. It must be found again by plugin and `sourceId`, also after a reload and on another client.
- Thread search: not run in this autonomous iteration.
- Advisor verdict (TASK-010 reviewer, claude-opus-5-5 xhigh): accept. Keying the handler on plugin and `sourceId` is the only option that survives a reload; a separate opener API repeats the same identity. It suggested letting `onOpen` return `false` to fall back to `item.url`; not adopted in v1.

## Decision

- `onOpen(item)` is an optional field of an attachment source. A press on a `plugin_resource` chip calls the `onOpen` of the source with the chip's plugin and `sourceId`. With no `onOpen`, the press opens `item.url`, as before.
- `pickerTitle`, `searchPlaceholder`, and `search` become optional. A source without `search` is chip-only: it stays out of the picker. A source with `search` still needs all three.
- A source needs `search` or `onOpen`.

## Consequences

- The cite plugin registers one source and gets no empty picker entry.
- The type change only loosens fields, so existing plugins still compile and load.
- `searchPluginAttachments` now throws for a source without `search`; the picker never calls it for one.

## Options Considered

- Keep the picker fields required: rejected. The cite plugin would need a search RPC and an empty picker entry to get a press handler.
- A separate `addComposerAttachmentOpener({ sourceId, onOpen })` API: rejected. It adds a second concept for the same chip identity.
- An `onOpen` on `addComposerAttachment`: rejected. It is lost on reload and on other clients.

## Revisit If

- A plugin needs different press behavior for chips of one source.
- Upstream Paseo adds its own chip press API.
