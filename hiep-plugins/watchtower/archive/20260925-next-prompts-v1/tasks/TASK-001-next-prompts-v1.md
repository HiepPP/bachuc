# TASK-001 Next prompts v1

Group: plugin (shared contract, server, and client)
Class: risky

## Brief

Goal: Render structured suggestions and reject undeclared or conflicting bulk sends.

Change: Replace unrestricted bulk sends with explicit selection rules.

How:

- Reproduce unsupported structured fences and unsafe legacy bulk sends with failing tests.
- Add strict versioned parsing, reference validation, and shared selection validation.
- Render radio groups, optional checkboxes, and a shared selection action bar.
- Preserve single sends, exact prompt text, Git controls, stale guards, and cleanup.
- Verify the plugin, reload it, then update the global Claude/Codex policy.

Files:

- [Shared modules](../../plugins/next-prompt-actions/shared/) define parsing and selection contracts.
- [Engine](../../plugins/next-prompt-actions/server/engine.ts) validates sends.
- [Client](../../plugins/next-prompt-actions/client/web.ts) renders selection controls.
- [Tests](../../plugins/next-prompt-actions/tests/) cover parsing, sends, and UI behavior.
- [README](../../plugins/next-prompt-actions/README.md) documents the contract.
- [Global policy](/Users/hiep/.claude/CLAUDE.md) changes after verification.

Expected result:

- Valid combinations send only prompt text in declared prompt order.
- Invalid JSON, versions, references, exclusions, and combinations fail closed.
- Legacy prompts cannot bypass the new bulk-send gate.
- Selection controls expose clear counts, keyboard access, and busy states.

Anti-goal: Preserve all existing single-send, Git, stale, and deduplication tests with zero failures. Measure before implementation, after integration, and at completion. Global policy stays unchanged until plugin verification passes.

## Verify

- Run focused RED tests before implementation.
- Run `npm run format`, `npm run typecheck`, lint, and `npm test` in the plugin directory.
- Inspect real rendered controls in light, dark, and narrow layouts with a synthetic fixture.
- Reload the installed plugin and confirm status, logs, and a functional RPC.
- Check the final diff and preserve the global Codex symlink.
