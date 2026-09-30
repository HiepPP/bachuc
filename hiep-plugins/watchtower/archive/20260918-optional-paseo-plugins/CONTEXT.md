# Plan Context

## Shared Context

- TASK-001, TASK-005, TASK-007, TASK-008 and TASK-009 are complete. Preflight discovery and optional Jev integration are installed.
- The user will select task IDs before any implementation starts.
- A general implement request must first resolve the selected scope. Do not implement the entire backlog by default.
- Each plugin lives in its own directory and remains independently installable.
- Group A covers shared plugin files and the root catalog. This does not require selecting tasks together.
- Use the installed paseo-plugin skill when implementing a selected task.
- Check the target daemon and app versions against matching plugin documentation before choosing APIs.
- TASK-005 uses the installed 0.8.0 SDK. The local daemon and app are version 0.8.0.
- Current plugin documentation was retrieved during implementation.
- Keep credentials and local artifacts out of Git. Preserve the existing Jev plugin.
- Use supported SDK operations and server-side RPC for filesystem, process, and external API access.
- Return cleanup for subscriptions, timers, and owned processes.
- Do not commit or push without a separate user request.
- TASK-005 was installed and reloaded for its authorized runtime verification.

- TASK-007 follows the [dashboard reference](../../../../watchtower/media/watchtower-dashboard.png), adapted to Paseo themes and layouts.
- The dashboard remains read-only. Task details hold long blocker text.

- TASK-008 opens Watchtower only in Explorer and labels the current project.

## Decisions

- Keep all proposals as TODO until the user selects work.
- TASK-009 extends completed TASK-001 and passed offline, synthetic live and native UI acceptance. Other proposals remain unselected.
- Suggested priority: workspace-preflight, evidence-ledger, then handoff-pack.

## Preflight and Jev

- TASK-009 adds discovery and MCP access to preflight, with agent-mediated Jev evaluation.
- Existing explicit settings remain supported; missing settings enable bounded metadata discovery.
- Preflight measures the environment. Jev evaluates relevance to the task and selects a suggested next step.
- Model answers never overwrite observations, execute repairs, or grant permission.
- Preserve independent plugin installations. No cross-plugin source imports or duplicated gateway credential handling.
- Planning TASK-009 does not authorize implementing unrelated TODO proposals.

## Open Decisions

- Which remaining task IDs should be implemented?
- Confirm the target host and any external service access when a selected task needs them.

## References

- [Repository rules](../../../AGENTS.md)
- [Plugin catalog](../../../README.md)
- [Existing Jev plugin](../../../plugins/jev-evaluator/README.md)
