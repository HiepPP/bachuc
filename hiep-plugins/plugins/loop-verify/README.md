# Loop verify

Prototype. Keeps an agent working on a goal until a shell command passes.

Label an agent with the command. After each turn the plugin runs it in the agent's
directory. On exit 0 the loop stops. On failure it starts a fresh child agent with the
original goal and the command's last output, up to 5 rounds.

```sh
paseo run --provider claude/haiku --mode bypassPermissions \
  --label 'loop-verify=npm test' \
  --label 'loop-verify-max=3' \
  "Make the failing test in tests/sum.test.ts pass."
```

| Label             | Meaning                                              |
| ----------------- | ---------------------------------------------------- |
| `loop-verify`     | Shell command run with `/bin/sh -c` after each turn. |
| `loop-verify-max` | Optional round limit, 1 to 10. Default 5.            |

Each round is a new agent, parented to the first one, so context never grows across
rounds. Children reuse the root's provider, model, mode, and thinking option.

- If verify fails and the agent's last message ends with a question, the loop pauses
  instead of starting a round. Reply to that agent; its next turn counts as the same
  round and is verified again.
- A canceled turn or archiving the root or current round stops the loop.
- A finished loop does not re-arm on follow-up turns to the root.
- The verify command times out after 15 minutes.
- Each verify result appends a **Loop verify** row to the end of the root agent's timeline
  and to the round's own agent: round, command, exit code, status, and failing output.
  Rows live in daemon memory and do not survive a daemon restart.
- Status also goes to `paseo plugin logs loop-verify` and
  `$PASEO_HOME/plugin-data/loop-verify/state.json`.

The verify command runs unsandboxed with the daemon user's access. Only label agents
with commands you would run yourself.

## Development

```sh
npm install
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id loop-verify
```
