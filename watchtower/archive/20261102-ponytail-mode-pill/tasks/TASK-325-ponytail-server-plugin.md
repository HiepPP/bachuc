# TASK-325 Ponytail server plugin

Group: A (writes `hiep-plugins/plugins/ponytail/**` only)
Class: risky

## Brief

Goal: A new Paseo plugin `ponytail` keeps one Ponytail mode per agent and adds the Ponytail rules to each Claude and Codex turn. It also ships a bridge hook and an installer for Claude sessions outside Paseo.

Change: no Ponytail anywhere -> plugin `ponytail` with mode RPCs, an `agent.prompt` hook, a bridge hook, and an installer.

Boundaries:

- Follows [ADR-0079](watchtower/decisions/ADR-0079-ponytail-plugin-over-extending-prompt-translate.md): copy patterns from prompt-translate, import nothing from it.
- Follows [ADR-0080](watchtower/decisions/ADR-0080-disabled-native-ponytail-with-bridge-hook.md): the daemon hook serves Paseo agents; the bridge serves only Claude outside Paseo.
- Default mode is `full` as a constant `DEFAULT_MODE`. No settings record and no settings screen.
- No React client. The React client is frozen; there is no `index.client.tsx`.
- A failed Ponytail step never fails the turn. Log a warning and send the turn without context, as prompt-translate does.
- Hiep chose on 2026-11-02: the daemon sends the full ruleset when the agent's mode differs from the last mode sent in this plugin process, and on every 10th turn since the last full ruleset. Other turns get a one-line reminder. The 10th-turn resend limits rule loss after a compaction or resume, which the daemon cannot see.
- The last-sent mode and the turn count live in an in-memory map per agent, so a restart sends the full ruleset again.
- Planner call: the daemon parses an explicit `/ponytail` or `$ponytail` command at the start of the prompt, saves the new mode, and uses it for that turn. A bare `/ponytail` means `full`.
- Planner call: subagents of a Paseo agent get no Ponytail rules ([ADR-0080](watchtower/decisions/ADR-0080-disabled-native-ponytail-with-bridge-hook.md) Consequences).
- Planner call: the bridge runs the native scripts unchanged, with their native side effects (mode flag, codebase map, statusline nudge). Plain Claude gets the same behavior as the enabled native plugin.
- `getPonytailInstructions` never throws on a missing SKILL.md; it falls back to built-in rules (`hooks/ponytail-instructions.js`). Only a missing `hook-runtime.json` or a failed `require` skips the context.
- Copy the seeding rule of `seedDraftPrefs` in [hiep-plugins/plugins/prompt-translate/server/draft.ts](hiep-plugins/plugins/prompt-translate/server/draft.ts): always delete the draft env key from `request.env`; seed only when `reason` is `create`, `purpose` is `interactive`, and no mode is saved.
- `agent.prompt` has `kind` `turn` or `steer`. Only `turn` gets context.
- Trap: `process.execPath` in the daemon is the Electron helper in the desktop app. Load `ponytail-instructions.js` in-process with `createRequire`; do not spawn it. If a spawn is needed, set `ELECTRON_RUN_AS_NODE=1`, as `turnContext` in [hiep-plugins/plugins/prompt-translate/server/turn.ts](hiep-plugins/plugins/prompt-translate/server/turn.ts) does.
- Trap: the bridge must never exit with code 2. Exit 2 blocks the user's Claude prompt. On error, write one stderr line and exit 1.
- Trap: the plugin bundle has no `import.meta`. Find the bridge and plugin files from `config.json` `plugins.ponytail.path`, as `contribute` in [hiep-plugins/plugins/prompt-translate/index.server.ts](hiep-plugins/plugins/prompt-translate/index.server.ts) does.

How:

- `shared/`: `ponytailModeSchema`, `DEFAULT_MODE`, the two RPC contracts, and `draftEnvKey` with `draftPrefsSchema`.
- `server/modes.ts`: `AgentModes` with atomic writes, copied in shape from prompt-translate `AgentModes`. `server/turn.ts`: `parseCommand` and `turnContext`.
- `server/draft.ts`: `seedDraftPrefs`, copied in shape from prompt-translate. It saves the draft mode, else `DEFAULT_MODE`.
- `index.server.ts`: handle both RPCs; `before("agent.session_open")` calls `seedDraftPrefs`; `before("agent.prompt")` builds the context for `claude` and `codex` turns, appends it, and writes `last-hook.json`.
- `server/ponytail-hook.cjs` bridge and `scripts/install-hooks.mjs` installer, shaped like the prompt-translate pair.

Files:

- [hiep-plugins/plugins/ponytail/paseo-plugin.json](hiep-plugins/plugins/ponytail/paseo-plugin.json) (new; id `ponytail`, `requirements.paseo` `>=0.10.2-beta.900`)
- [hiep-plugins/plugins/ponytail/package.json](hiep-plugins/plugins/ponytail/package.json) (new; scripts `typecheck`, `test`, `lint`, `format` as in prompt-translate, without React deps)
- [hiep-plugins/plugins/ponytail/package-lock.json](hiep-plugins/plugins/ponytail/package-lock.json) (new; from `npm install`)
- [hiep-plugins/plugins/ponytail/tsconfig.json](hiep-plugins/plugins/ponytail/tsconfig.json) (new)
- [hiep-plugins/plugins/ponytail/index.server.ts](hiep-plugins/plugins/ponytail/index.server.ts) (new; RPC handlers and the two `before` hooks)
- [hiep-plugins/plugins/ponytail/shared/modes.ts](hiep-plugins/plugins/ponytail/shared/modes.ts) (new; schema, default, draft key)
- [hiep-plugins/plugins/ponytail/shared/contracts.ts](hiep-plugins/plugins/ponytail/shared/contracts.ts) (new; the two RPCs)
- [hiep-plugins/plugins/ponytail/server/modes.ts](hiep-plugins/plugins/ponytail/server/modes.ts) (new; `AgentModes`)
- [hiep-plugins/plugins/ponytail/server/turn.ts](hiep-plugins/plugins/ponytail/server/turn.ts) (new; `parseCommand`, `turnContext`, `readRuntime`)
- [hiep-plugins/plugins/ponytail/server/draft.ts](hiep-plugins/plugins/ponytail/server/draft.ts) (new; `seedDraftPrefs`)
- [hiep-plugins/plugins/ponytail/tests/draft.test.ts](hiep-plugins/plugins/ponytail/tests/draft.test.ts) (new; seeding with a draft, without a draft, and on `resume`)
- [hiep-plugins/plugins/ponytail/server/ponytail-hook.cjs](hiep-plugins/plugins/ponytail/server/ponytail-hook.cjs) (new; bridge)
- [hiep-plugins/plugins/ponytail/scripts/install-hooks.mjs](hiep-plugins/plugins/ponytail/scripts/install-hooks.mjs) (new; installer and `--remove`)
- [hiep-plugins/plugins/ponytail/tests/turn.test.ts](hiep-plugins/plugins/ponytail/tests/turn.test.ts) (new; red tests below)
- [hiep-plugins/plugins/ponytail/tests/modes.test.ts](hiep-plugins/plugins/ponytail/tests/modes.test.ts) (new; `AgentModes` default and round trip)
- [hiep-plugins/plugins/ponytail/README.md](hiep-plugins/plugins/ponytail/README.md) (new; what it does, install, remove)

Interfaces:

- Produces: RPC `ponytail.mode.read` with input `{ agentId: string (uuid) }` and output `{ mode: "off" | "lite" | "full" | "ultra" }` for TASK-326
- Produces: RPC `ponytail.mode.write` with input `{ agentId: string (uuid), mode: "off" | "lite" | "full" | "ultra" }` and output `{ mode: "off" | "lite" | "full" | "ultra" }` for TASK-326
- Produces: draft composer value under plugin id `ponytail`: `{ mode: "off" | "lite" | "full" | "ultra" }`, read from env `PASEO_PLUGIN_COMPOSER_ponytail` at `agent.session_open` for TASK-326
- Produces: file `hiep-plugins/plugins/ponytail/scripts/install-hooks.mjs`, run as `node scripts/install-hooks.mjs <ponytailRoot>` or `node scripts/install-hooks.mjs --remove`, for TASK-327
- Produces: file `$PASEO_HOME/plugin-data/ponytail/agents/<agentId>/last-hook.json` with fields `mode` and `kind` (`rules`, `reminder`, or `off`) for TASK-327

Interfaces note: `turnContext` takes `{ ponytailRoot: string; mode: PonytailMode; lastMode: PonytailMode | undefined; turnsSinceRules: number }` and returns `string | null`.

Expected result:

- WHEN `ponytail.mode.read` runs for an agent with no saved mode THE plugin SHALL return `{ mode: "full" }`.
- WHEN `ponytail.mode.write` saves a mode THE plugin SHALL return it and a later read SHALL return the same mode.
- WHEN a Claude or Codex turn runs and the agent's mode is not `off` and differs from the last mode sent THE hook SHALL append a header naming the mode plus `getPonytailInstructions(mode)`.
- WHEN the agent's mode equals the last mode sent and 9 reminder turns followed the last full ruleset THE hook SHALL append the full ruleset again.
- WHILE the agent's mode equals the last mode sent and fewer than 9 reminder turns followed the last full ruleset, WHEN a turn runs THE hook SHALL append one reminder line naming the mode.
- WHEN the mode turns `off` after an active mode THE hook SHALL append one line that tells the agent to stop following earlier Ponytail rules, and later `off` turns SHALL get nothing.
- WHEN the prompt starts with `/ponytail lite`, `/ponytail ultra`, `/ponytail off`, or a bare `/ponytail` THE hook SHALL save `lite`, `ultra`, `off`, or `full` before it builds the context.
- WHEN a new interactive agent opens THE plugin SHALL save the draft mode, or `full` when there is no draft value, and SHALL remove the draft env key.
- IF the session opens with a `reason` other than `create` THEN THE plugin SHALL keep any saved mode and only remove the draft env key.
- IF `hook-runtime.json` is missing or the `require` of `ponytail-instructions.js` fails THEN THE turn SHALL go out unchanged and the daemon SHALL log one warning.
- IF the provider is not `claude` or `codex`, or the request kind is not `turn`, THEN THE hook SHALL change nothing.
- WHEN the bridge gets a hook input and `PASEO_AGENT_ID` is a UUID THE bridge SHALL print nothing and exit 0.
- WHEN the bridge gets a `SessionStart`, `UserPromptSubmit`, or `SubagentStart` input with no `PASEO_AGENT_ID` THE bridge SHALL run `hooks/ponytail-activate.js`, `hooks/ponytail-mode-tracker.js`, or `hooks/ponytail-subagent.js` from `ponytailRoot` with `CLAUDE_PLUGIN_ROOT` set, and print its output.
- WHEN the installer runs twice with the same root THE `~/.claude/settings.json` hooks SHALL hold one bridge entry per event, and `--remove` SHALL delete only those entries.

Red tests:

File: [hiep-plugins/plugins/ponytail/tests/turn.test.ts](hiep-plugins/plugins/ponytail/tests/turn.test.ts)

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { parseCommand, turnContext } from "../server/turn";

function fakeRoot(): string {
  const root = mkdtempSync(path.join(tmpdir(), "ponytail-"));
  mkdirSync(path.join(root, "hooks"));
  writeFileSync(
    path.join(root, "hooks/ponytail-instructions.js"),
    "module.exports = { getPonytailInstructions: (mode) => `RULES ${mode}` };",
  );
  return root;
}

test("parseCommand reads explicit levels and ignores other prompts", () => {
  assert.equal(parseCommand("/ponytail lite\nfix it"), "lite");
  assert.equal(parseCommand("$ponytail ultra"), "ultra");
  assert.equal(parseCommand("/ponytail off"), "off");
  assert.equal(parseCommand("/ponytail"), "full");
  assert.equal(parseCommand("/ponytail-review"), null);
  assert.equal(parseCommand("please use /ponytail lite"), null);
  assert.equal(parseCommand("fix the bug"), null);
});

test("turnContext sends the rules on a mode change and a reminder after", () => {
  const ponytailRoot = fakeRoot();
  const first = turnContext({
    ponytailRoot,
    mode: "full",
    lastMode: undefined,
    turnsSinceRules: 0,
  });
  assert.ok(first?.includes("RULES full"));
  const again = turnContext({ ponytailRoot, mode: "full", lastMode: "full", turnsSinceRules: 1 });
  assert.ok(again && !again.includes("RULES") && again.includes("full"));
  assert.ok(Buffer.byteLength(again) <= 300);
  const changed = turnContext({
    ponytailRoot,
    mode: "ultra",
    lastMode: "full",
    turnsSinceRules: 3,
  });
  assert.ok(changed?.includes("RULES ultra"));
});

test("turnContext resends the rules after 9 reminder turns", () => {
  const ponytailRoot = fakeRoot();
  const eighth = turnContext({ ponytailRoot, mode: "lite", lastMode: "lite", turnsSinceRules: 8 });
  assert.ok(eighth && !eighth.includes("RULES"));
  const tenth = turnContext({ ponytailRoot, mode: "lite", lastMode: "lite", turnsSinceRules: 9 });
  assert.ok(tenth?.includes("RULES lite"));
});

test("turnContext turns off once, then stays silent", () => {
  const ponytailRoot = fakeRoot();
  const stop = turnContext({ ponytailRoot, mode: "off", lastMode: "full", turnsSinceRules: 2 });
  assert.ok(stop && !stop.includes("RULES"));
  assert.equal(
    turnContext({ ponytailRoot, mode: "off", lastMode: "off", turnsSinceRules: 9 }),
    null,
  );
  // After a restart the first off turn still tells the agent to stop.
  assert.ok(turnContext({ ponytailRoot, mode: "off", lastMode: undefined, turnsSinceRules: 0 }));
});
```

Anti-goal: the context added to a turn that repeats the last mode stays at or under 300 bytes; tripwire; read from the `turnContext` return length in a test at each material change and at completion.

## Verify

- `cd hiep-plugins/plugins/ponytail && npm install && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/ponytail && npm test` -> all tests pass, including the four red tests above, the `AgentModes` default test, and the three `seedDraftPrefs` cases.
- `cd hiep-plugins/plugins/ponytail && npm run lint && npm run format` -> exit 0, no diff left by format.
- `printf '{"hook_event_name":"SessionStart"}' | PASEO_AGENT_ID=00000000-0000-4000-8000-000000000000 node hiep-plugins/plugins/ponytail/server/ponytail-hook.cjs --claude` -> empty stdout, exit 0.
- Installer test with `CLAUDE_CONFIG_DIR` and `PASEO_HOME` set to temp folders and a fake root with `hooks/ponytail-instructions.js`: run it twice, then `--remove` -> one bridge entry per event after two runs; none after remove; other hooks unchanged.
- `printf '{"hook_event_name":"UserPromptSubmit","prompt":"/ponytail lite"}' | env -u PASEO_AGENT_ID CLAUDE_CONFIG_DIR=<temp> PASEO_HOME=<temp with hook-runtime.json pointing at /tmp/ponytail-src/repo> node hiep-plugins/plugins/ponytail/server/ponytail-hook.cjs --claude` -> exit 0 with the native tracker output.
- The bridge with a `PASEO_HOME` that has no `hook-runtime.json` and no `PASEO_AGENT_ID` -> exit 1 and one stderr line, never exit 2.
- Anti-goal: the red test asserts the reminder is at most 300 bytes -> passes.
