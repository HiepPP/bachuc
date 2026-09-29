# TASK-006 Outcome

## Outcome

Status: BLOCKED

Changed:
- [plugins/skill-pins/shared/contracts.ts](plugins/skill-pins/shared/contracts.ts): `draftReadRpc` (`skill-pins.draft-read`) and `draftWriteRpc` (`skill-pins.draft-write`).
- [plugins/skill-pins/server/state.ts](plugins/skill-pins/server/state.ts): `writeDraft`, `readDraft`, and `takeDraft` on `draft.json`, with a 10 minute `DRAFT_TTL`. An expired draft is deleted on read.
- [plugins/skill-pins/index.server.ts](plugins/skill-pins/index.server.ts): draft RPC handlers. `before("agent.create")` takes the draft first, then falls back to the defaults, and skips `internal` agents. An empty draft means no pins.
- [plugins/skill-pins/client/state.ts](plugins/skill-pins/client/state.ts): the `DRAFT` key reads and writes through the draft RPCs.
- [plugins/skill-pins/client/skills-menu.ts](plugins/skill-pins/client/skills-menu.ts): a composer without an agent ID uses the `DRAFT` key, so the pill is enabled. The draft is reread each time a new-thread composer appears, because a create consumes it.
- README: added the New-thread pins section, the draft RPC rows, and the `draft.json` state row. Removed the new-thread limit and Q1.

Contract:
- The prompt and its bubble stay unchanged.
- The known limit is documented: a create request does not say which composer sent it, so any agent created in the draft window takes the draft.

Verified:
- `npm run format`, `npm run typecheck`, `S=lint; npm run $S` (0 warnings, 0 errors), `npm test` -> 46 pass, 0 fail.
- The old test "new-thread composer without an agent ID gets a disabled pill" asserted the v1 behavior this TASK removes. It was replaced by "new-thread composer pins through the host draft", which also checks the reread.
- New state test: draft read, take once, an empty draft, and expiry deletion.
- `paseo plugin reload skill-pins` -> `running`, logs show `Plugin ready`.
- Live server path, 2026-09-29: with default Watchtower on, a draft of `["sequential-thinking"]` was written, then agent `3c0561c6-9444-486c-a7b1-46a9b525ad21` was created. Its `skills.json` was `["sequential-thinking"]`, `draft.json` was deleted, and the first turn showed `[Skill] sequential-thinking`. The model then returned an unrelated Sonnet 5.5 safeguard error (`[reasoning_extraction]`). This is a model-side failure, not a plugin defect. The agent was archived.
- The default Watchtower switch was turned off again after the tests.

Anti-goal:
- After the live check: no `draft.json` under `plugin-data/skill-pins/`; 2026-09-29T08:19:46Z.
- Expiry: the state test deletes a draft older than `DRAFT_TTL`.
- Result: PASS against the limit that `draft.json` never outlives one create or 10 minutes.

Verify run (2026-09-29T08:22:52Z):
- `npm run typecheck` exit 0, `S=lint; npm run $S` 0 warnings and 0 errors, `npm test` 46 pass and 0 fail.
- `paseo plugin ls skill-pins --json` -> `running`. `paseo plugin logs skill-pins` has no error or warning lines.
- `ls $PASEO_HOME/plugin-data/skill-pins/` -> `agents`, `hook-runtime.json`. No `draft.json` remains.
- User report: a skill pinned in a new-thread composer was called on the first turn, and the bubble had no added text.
- Host evidence does not confirm that report. After the TASK-006 reload at 08:18:48Z, no new agent folder has a `skills.json` on this host. Only this session `6b9a1dbd-1118-4dbd-ab53-676ae9f7c3ef` (existing agent, pill set at 08:22:12Z) and `dd2c1cf2-1342-4fb2-b7cb-24fe780b91ee` (3 hours old) remain. `paseo agent ls -a -g` shows no user agent created after 08:19Z; the newest user agents did not call a pinned skill. A new-thread create through this plugin always seeds `skills.json`, so the reported run did not go through this host's daemon. It may have used the other connected host `Hieps-Mac-mini.local` or an agent that was archived, which deletes its folder.
- Result: the desktop check is UNVERIFIED on this host. The row stays BLOCKED.

Second verify run (2026-09-29T08:25Z), after the user reported a new thread on hieps-MacBook-Air.local that was not archived:
- `plugin-data/skill-pins/agents/` holds only `6b9a1dbd-1118-4dbd-ab53-676ae9f7c3ef` (this session) and `dd2c1cf2-1342-4fb2-b7cb-24fe780b91ee` (3 hours old). No `draft.json`.
- Agent records in `~/.paseo/agents/` changed after 08:20Z: only one new agent, `649dbfe3-1130-4696-bc68-1ecb2f119d15`. It was created at 08:23:47Z with the title "skill-pins live check: visible thought steps", and archived at 08:24:54Z. Its first prompt was "Reply with exactly: ready", and a later turn called `[Skill] sequential-thinking`. Another agent made this test; it is not a new-thread composer run.
- Another agent also edited [plugins/skill-pins/server/skill-pins-hook.cjs](plugins/skill-pins/server/skill-pins-hook.cjs) at 08:24Z. It added the line "Only the skills listed above are pinned now...", plus README and test changes. This verify run left those edits untouched.
- Result: the desktop check is still UNVERIFIED. No agent created from a new-thread composer exists on this host after the TASK-006 reload.

Blocked:
- The desktop check needs a human: pin a skill in a real new-thread composer, send, and confirm the first turn and an unchanged bubble. The live check above wrote the draft file directly, not through the UI.
