# ADR-0035 Generated Rust protocol types over hand-written types

Status: accepted
Date: 2026-10-15
Supersedes: -
Scope: `packages/desktop-gpui/crates/protocol/**`

## Context

- [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) has 7741 lines and about 430 message types in the zod schemas `WSInboundMessageSchema` and `WSOutboundMessageSchema`.
- The repo uses zod 4.4.3, which has `z.toJSONSchema`.
- The protocol adds optional fields often, per [docs/protocol-compatibility.md](docs/protocol-compatibility.md). A Rust client must follow these changes.

## Decision

- Export JSON Schema from the zod schemas with a Node script, and generate the Rust types from that schema.
- The owner chose this in the planning interview on 2026-10-15.

## Consequences

- A protocol change reaches Rust by running the export script and rebuilding, not by editing Rust by hand.
- The generated code can be large and ugly for big unions. The crate exposes a few stable names on top of it.
- The export script needs a built `packages/protocol`, so `npm run build:client` runs before it.

## Options Considered

- Hand-written serde structs for the subset the client uses, with JSON fixture tests: rejected. It was simpler to start, but every protocol change needs a manual Rust edit, and drift shows only at runtime.

## Revisit If

- The generator cannot produce compiling Rust for the session message unions after a fix attempt.
- The generated module makes a clean build of the protocol crate take more than 3 minutes.
