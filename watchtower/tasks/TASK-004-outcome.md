# TASK-004 Outcome

## Outcome

Status: DONE

Changed:
- Installed the plugin with `paseo plugin install`. The first load failed with "Plugin contribution must return a cleanup function". [plugins/skill-pins/index.server.ts](plugins/skill-pins/index.server.ts) now returns `() => {}`, and the reload succeeded.
- With user approval, ran `node scripts/install-hooks.mjs /Users/hiep/.claude/plugins/cache/claude-plugins-official/superpowers/6.3.0/skills`. It registered one `UserPromptSubmit` entry in `~/.claude/settings.json` (`--claude`) and one in `~/.codex/hooks.json` (`--codex`).
- Added the skill-pins row to the root [README.md](README.md) catalog. The count now says Fourteen, matching the 14 table rows. The old text said Eleven, which was already wrong before this change.

Contract:
- The hook returns `{}` for an agent with no pinned skills and writes no `last-hook.json`.

Verified:
- `paseo plugin ls skill-pins --json` -> `running`, no error.
- `paseo plugin logs skill-pins` -> `Plugin ready` after the fix. No errors after the test turns.
- Live Claude agent `192a9d30-3984-44c1-b73e-2f622feba730` (Sonnet 5, low thinking). The first turn had no pins and produced no `last-hook.json`. Brainstorm and Verify were then pinned with `SkillPins.set`, the same code path as `skillsWriteRpc`.
- Second turn -> `last-hook.json` showed `skills: ["brainstorming","verification-before-completion"]`, `source: "agent"`, `contextBytes: 322`, and no prompt text.
- Second turn timeline -> `[Skill] superpowers:verification-before-completion` was called before the answer. `superpowers:brainstorming` was not called.
- The test agent was archived. Its state folder `$PASEO_HOME/plugin-data/skill-pins/agents/192a9d30-3984-44c1-b73e-2f622feba730/` was left in place.

Desktop visual check (2026-09-29, Paseo desktop light theme, this agent's composer, compared with [plugins/skill-pins/artifacts/u1-light.png](plugins/skill-pins/artifacts/u1-light.png) and [plugins/skill-pins/artifacts/u1-dark.png](plugins/skill-pins/artifacts/u1-dark.png)):
- K1 Empty: `Skills` is dimmed with no background. Matches the mockup.
- K2 One pin: the label is `Watch` with a tinted background, and the title is `Pinned: Watchtower`. The menu stays open, shows the check mark, and shows `Bỏ chọn tất cả`. Matches.
- K3 Three pins: the label is `Watch · Goal · Seq` with a tint. The title lists all three names. Matches.
- K4 Open menu: the header, descriptions, amber warning on Chase goal, divider, and `Bỏ chọn tất cả` all render. The layout matches.
- D1: A mouse click leaves a 2px outline on the clicked option, because the style used `:focus`. The mockup shows that outline only for keyboard focus. Fixed on 2026-09-29 by using `:focus-visible`; not yet rechecked on the desktop.
- D2: The menu is anchored at its bottom edge. When the first pin makes `Bỏ chọn tất cả` appear, the whole menu grows upward by one row. The static mockup cannot show this. Not fixed.
- D3: Text is larger than in the mockup, because the pill copies the host model selector's typography instead of the mockup's 13px. This is expected.
- Not checked: the dark theme on the desktop, clicking `Bỏ chọn tất cả` from the UI, and closing the menu with Escape. The user stopped UI actions before these steps. This agent's `skills.json` was `[]` afterward, and the hook at 06:39 reported `contextBytes` 0.

Live check with the stricter Claude wording (2026-09-29):
- Agent `96780f16-2076-4459-b05d-d58b7de9e6db` (Claude Sonnet 5, low thinking). The first turn had no pins. Then `SkillPins.set` pinned `watchtower` and `sequential-thinking`.
- Second turn -> `last-hook.json` showed `skills: ["watchtower","sequential-thinking"]`, `source: "agent"`, `contextBytes: 290`, at 07:26:59Z.
- Second turn timeline -> `[Skill] watchtower` and `[Skill] sequential-thinking` were both called before the answer.
- Result: the skipped-skill blocker from the first live check did not repeat after "still applies" was removed. This is one run, so compliance is still probabilistic.
- The test agent was archived. Its state folder under `$PASEO_HOME/plugin-data/skill-pins/agents/` was left in place.

Dark theme check (2026-09-29, agent run through cua_repl, then the theme was switched back to Light):
- K5 `Bỏ chọn tất cả`: PASS. All check marks cleared, the pill label went back to `Skills`, and the menu stayed open.
- K6 D1 `:focus-visible`: PASS. A mouse click on Watchtower showed only the hover background, with no 2px outline.
- D4: in dark theme, the skill names and descriptions are dark gray on a dark menu background. They are hard to read. Not fixed; the user chose to skip dark theme.
- D5: with no pins, the `Bỏ chọn tất cả` row stays visible but dimmed. This is by design: [plugins/skill-pins/client/skills-menu.ts](plugins/skill-pins/client/skills-menu.ts) sets `disabled` on it while nothing is pinned.
- The clear-all test removed this chat's `sequential-thinking` pin.

User decisions (2026-09-29):
- The user verified R1 (Escape closes the menu) and R4 (D2 menu growth) on the desktop.
- The user skipped the dark theme, the queued prompt, the Caveman menu check, and Codex hook trust. The TASK is closed with these checks waived, not passed.

Blockers:
- Resolved 2026-09-29: the first live check skipped brainstorming under the "still applies" wording. The stricter wording called both pinned skills in the second live check.
- Manual desktop checks still need a human: the pill UI, a queued prompt, and the Caveman menu next to the pill.
- Codex is not verified. The hook must first be trusted in Codex `/hooks`.

Anti-goal:
- Right after install: other keys in `~/.claude/settings.json` and `~/.codex/hooks.json` are identical to the `.skill-pins-backup` files (`jq -S 'del(.hooks.UserPromptSubmit)'` diff was empty). The only `UserPromptSubmit` change is one added skill-pins entry in each file. 2026-09-29 11:56.
- Result: PASS against the limit that nothing except the added entries changes.
