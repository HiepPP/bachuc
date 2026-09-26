# Native subagent routing adapter

The plugin injects native routing only into interactive Codex/Claude sessions opened by Paseo.
The existing `/route` panel and `delegate_task` remain separate paths. Runtime evidence and historical failures are recorded in VERIFICATION.md.

Run `npm run native:install` once, then reload `jev-orchestrator`. The installer preserves existing
hook groups, writes local backups, and registers one guarded `PreToolUse` command in each runtime.
Without either Paseo marker, an ordinary outside session does not get routed. A partially present
marker fails closed. History-only sessions do not receive markers. Existing running provider
processes need reopening; source reload alone cannot change their environment.

The editable allowlist is `$PASEO_HOME/plugin-data/jev-orchestrator/native/settings.json`.
It contains `enabled` and `pairs.codex`/`pairs.claude` arrays of `{model, effort, description}`.
Changes apply on the next Paseo session open. Set `enabled: false` to stop injecting routing into
subsequent launches. No global model defaults or original agent definitions are edited.

The adapter handles Codex `spawn_agent` (`Agent` alias) and Claude `Agent`/legacy `Task` calls at
`PreToolUse`. It asks Jev once to select a configured custom-agent variant. Custom role variants pin
model and effort while retaining the original instructions, tools and permissions.
No Paseo source changes are required. Roles without a route pass through unchanged and keep the
provider's own definition. Direct use of a reserved `jev-native-*` type, resume requests, invalid
choices, evaluator errors and changed definition files return a deny that names the cause. There is
no automatic retry. A Claude call without `subagent_type` routes as `general-purpose`. Calls from a
subdirectory of the manifest workspace stay in scope.
Luna is accepted only with `max`; use canonical model IDs, not aliases hiding the underlying model.

Codex role files override explicit spawn settings, so the adapter changes `agent_type` and removes
explicit model/effort fields. Claude has no per-call effort field, and its `Agent` `model` field
accepts only aliases such as `sonnet`. The adapter changes `subagent_type` to a definition with the
pinned full model ID and effort, and removes any explicit `model` so the definition applies.
Other input fields are retained. This is a provider-level command hook, not an optional instruction
asking the main agent to call Jev.

## Configuration to review before activation

Keep the manifest outside Git. It is scoped to one workspace. Its candidate list is the persistent
allowlist for that runtime; Jev can only select among those candidates for the requested source role.

`server/native-preset.ts` supplies the user-selected initial pairs for each source role:

| Runtime | Clear parent-assigned specification | Verification or edge cases | Difficult or ambiguous work |
| ------- | ----------------------------------- | -------------------------- | --------------------------- |
| Codex   | `gpt-5.6-luna` / `max`              | —                          | `gpt-6-astra` / `low`       |
| Claude  | `claude-opus-5-5` / `medium`        | `claude-opus-5-5` / `high` | `claude-opus-5-5` / `xhigh` |

The Codex pairs appeared in the installed Paseo provider catalog on 2026-09-21. The Claude pairs
appeared there on 2026-09-27; the user dropped Fable 5.1 that day. A catalog check does not prove
native runtime execution. The preset descriptions tell Jev to consider actual
ambiguity and scope, rather than route mechanically because a prompt contains the word "spec".
The cheaper-execution preference is user policy; savings have not been benchmarked for this adapter.

```json
{
  "cwd": "/absolute/workspace",
  "policy": {
    "runtime": "codex",
    "routes": [
      {
        "sourceType": "worker",
        "candidates": [
          {
            "agentType": "jev-worker-luna-max",
            "model": "gpt-5.6-luna",
            "effort": "max",
            "description": "Describe the model's verified capabilities and intended work."
          }
        ]
      }
    ]
  },
  "definitions": [
    {
      "agentType": "jev-worker-luna-max",
      "path": "/absolute/path/to/jev-worker-luna-max.toml",
      "sha256": "REPLACE_WITH_SHA256_OF_REVIEWED_FILE"
    }
  ]
}
```

For Claude use `runtime: "claude"`, its source role names, supported Claude model IDs/efforts,
and Markdown agent definitions. Supply exactly one definition for every variant. Hashes are checked
before and after evaluation. They detect changes; they do not parse or certify the definitions.
Review the pinned values and preserved permissions before computing hashes. Confirm the provider
loads these exact definitions and no higher-precedence agent definition shadows them.

The registered command uses absolute paths to Node, the loader and script. Paseo supplies the
manifest in `PASEO_JEV_NATIVE_POLICY` and provider in `PASEO_JEV_NATIVE_RUNTIME`:

```sh
/absolute/node --import /absolute/jev-orchestrator/node_modules/tsx/dist/loader.mjs \
  /absolute/jev-orchestrator/server/native-hook-command.ts
```

Merge this command into the runtime's existing `PreToolUse` configuration with matcher
`^(spawn_agent|Agent|collaborationspawn_agent|mcp__jev_orchestrator__prepare_native_delegate)$` for Codex or `^(Agent|Task)$` for Claude, and a 30-second timeout. Preserve
existing hooks. The command reads the hook event from stdin and writes only hook JSON to stdout.
The command reads the existing Gateway credential from `$PASEO_HOME/config.json`, defaulting to
`~/.paseo/config.json`. Do not put credentials in hook configuration. Task text is sent to Jev;
the adapter does not save it. Evaluation uses the existing worker timeout and no retries.

## Activation and limits

- Codex requires review/trust of new hooks using `/hooks`; an untrusted hook is skipped. Do not bypass trust.
- Variants are generated in the user's native `agents` directories, with reserved `jev-native-*` names.
  They can appear in native agent catalogs outside Paseo, but automatic Jev selection remains scoped
  to Paseo-marked sessions. Their descriptions add context overhead that has not been benchmarked.
- The built-in Claude `Explore`/`Plan` substitutes allow only `Read`, `Grep`, and `Glob`; they are narrower
  than the original built-ins. Their internal prompts cannot be copied from public definitions.
  Other built-ins without an explicit mapping pass through unrouted. Frontmatter that strict YAML
  rejects is read as flat `key: value` lines, as Claude Code does; nested values or a missing name
  leave the definition unmapped. Plugin-provided role definitions are not discovered by the file scanner.
- Configure supported model/effort pairs and role variants separately for each provider. A Codex model
  in the Paseo catalog does not establish availability in Claude's native runtime.
- Check Claude's `availableModels` policy and `CLAUDE_CODE_SUBAGENT_MODEL_FORCE`: these can substitute
  or force a different model after routing. An input allowlist alone cannot certify the final model.
- Review other matching hooks: they may modify the same input. A command failing to start, a disabled
  hook or a host timeout may fail open. The adapter emits its own denial after 25 seconds, but cannot
  turn provider hook availability into an unconditional security boundary.
- This covers the documented tool boundary. It does not establish coverage of forks, skill-created
  sessions, teammates, other provider-internal paths, or already-running children.
- Verify actual child model/effort in provider session metadata before claiming active routing. Offline
  tests establish adapter behavior only. The first native v2 acceptance failed (see VERIFICATION.md);
  v2 requires the explicit preflight contract below.

References: [Codex hooks](https://learn.chatgpt.com/docs/hooks),
[Codex subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents),
[Claude hooks](https://code.claude.com/docs/en/hooks),
[Claude subagents](https://code.claude.com/docs/en/sub-agents).

## Codex native v2 limitation (0.154.0)

The initial real Paseo test bypassed the old matcher and created a Sol/high child outside the
allowlist. Codex flattens `collaboration.spawn_agent` to `collaborationspawn_agent`, without a dot.
The installer now migrates only its exact old single-command registration, preserving other hooks.
The changed registration must be reviewed/trusted again in Codex.

The v2 handler passes encrypted task messages through unchanged. Its hook event lacks the source
flag that distinguishes plaintext from encrypted messages. Never evaluate that ciphertext.

### Preflight and one-use ticket

On a **fresh Codex parent created through Paseo**, call `prepare_native_delegate` with a stable
`requestId`, complete plaintext `task` including acceptance criteria, `sourceRole`, `forkTurns: "none"`,
and `shareWithJev: true`. Its PreToolUse hook verifies a root Codex transcript and registers an intent
bound to the native session, Paseo parent and workspace. The MCP call must match that exact intent.

The server asks Jev once, chooses an allowlisted pair, and writes the original role plus the authorized
task into a dedicated role file. Codex caches role names at startup, so the plugin reserves **three
unique, non-reusable role slots per parent session** before opening the provider. It changes each
known file once before spawning; it never overwrites an active child's task. Capacity exhaustion,
full-history forks and descendant preflights are denied in this first implementation.

Use the returned `taskName` as native `task_name`, `sourceRole` as `agent_type`, `fork_turns: "none"`,
and the returned `message` unchanged. Do not pass model/effort or use `routedAgentType` directly.
The spawn hook atomically consumes the ticket, checks the unchanged policy/definitions, substitutes
its pinned role and removes explicit model/effort. No second evaluator call is made. The ticket name
uses only lowercase letters, digits and underscores, independently of the role's filename.

Tickets expire five minutes after intent registration. Repeated prepares deduplicate the same request;
changed payloads are rejected. Consumption is at most once, **not a promise of exactly one successful
child**: if Codex later rejects the spawn, the ticket stays consumed. Do not automatically resubmit with
a new ID. `native_delegation_status` reports evaluator usage, elapsed evaluation time and ticket state;
`consumed` is not task completion. Plugin reload, scope revocation or policy changes invalidate tickets.

### Routing failure recovery

`Paseo daemon transport disconnected` means the plugin's SDK connection failed, not a bad Jev
policy. After the plugin checks pass, run `paseo plugin reload jev-orchestrator`, verify its status
and logs, then create a fresh Paseo parent and verify `prepare_native_delegate` there. Do not
restart the daemon. Reload revokes existing parent bindings and tickets; it cannot repair an old
parent's injected MCP URL/token. `session binding expired or invalid` also requires a fresh parent.

`bridge unreachable` requires checking plugin availability. `ticket rejected` means the bridge
responded but rejected the operation: inspect `native_delegation_status`, the request contract,
workspace, policy, expiry and remaining slots. Never retry a failed task with a new request ID.
Unknown internal errors remain redacted; no failure allows delegation through.

Task text is sent to Jev and held in a local role file with mode 0600, outside Git. Parent archival
removes its known role slots. Reload/crash preserves files to avoid breaking running children, so
orphan slots may need explicit cleanup after those children finish. They may appear in later native
role catalogs until cleaned. Normal standalone sessions remain outside automatic routing.

The authorized task is supplied through developer instructions; the encrypted native message is only
a transport pointer. This does not cryptographically prove that ciphertext contains matching text,
and it is not a security boundary against hostile local processes or conflicting parent instructions.
Native child model/effort and task completion still require runtime verification. Provider guardian
review sessions are not delegation calls and are not selected by this ticket policy.

Version-pinned source:

- [Hook name selection](https://github.com/openai/codex/blob/rust-v0.154.0/codex-rs/core/src/tools/registry.rs#L799)
- [Namespace concatenation](https://github.com/openai/codex/blob/rust-v0.154.0/codex-rs/core/src/tools/mod.rs#L40)
- [Encrypted message handling](https://github.com/openai/codex/blob/rust-v0.154.0/codex-rs/core/src/tools/handlers/multi_agents_v2.rs#L57)

- [Known role files are reread at spawn](https://github.com/openai/codex/blob/rust-v0.154.0/codex-rs/core/src/agent/role.rs#L131)
