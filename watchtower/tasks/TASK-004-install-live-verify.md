# TASK-004 Install and live verify

Group: D (writes the root README catalog and real provider hook config; ordered after TASK-002 and TASK-003)
Class: risky

## Brief

Goal: Install the plugin and hook on this host, prove a pinned skill loads in a real agent turn, and list the plugin in the root README.

Change: tested code -> running plugin with a registered hook and a catalog entry.

Design: [plugins/skill-pins/README.md](plugins/skill-pins/README.md) (sections Install, Compatibility and limits)

Boundaries:

- Ask the user before running `node scripts/install-hooks.mjs`. It edits `~/.claude/settings.json` and `~/.codex/hooks.json`.
- Use the installed `paseo` CLI for the host from `paseo daemon status --json`. Never restart the daemon.
- Codex needs the user to trust the hook in `/hooks`. Existing agents need `paseo agent reload <id>`.
- Keep the root README wording short, like the other catalog entries. Update the plugin count.

How:

- Run `paseo plugin install "$PWD" --id skill-pins` from the plugin folder.
- After approval, run the installer.
- In Paseo desktop, pin Brainstorm and Verify on a new Claude agent and send a prompt.
- Read `last-hook.json` and the agent timeline.
- Repeat with no pins, then with a Codex agent if the user trusts the hook.
- Add the catalog entry to the root README.

Files:

- [README.md](README.md) (add the Skill pins catalog entry and update the plugin count)
- `~/.claude/settings.json` and `~/.codex/hooks.json` (hook entries added by the installer, outside Git)

Expected result:

- `paseo plugin ls skill-pins --json` shows `running` with no load error.
- A Claude turn with two pins calls the Skill tool for both skills before the answer.
- A turn with no pins has no skill-pins context, and `last-hook.json` shows `contextBytes` 0 or is not written.
- The Caveman menu and hook still work in the same composer.

Anti-goal: Other entries in `~/.claude/settings.json` and `~/.codex/hooks.json` stay byte-identical apart from the added skill-pins entries; tripwire; read by diffing each file against its `.skill-pins-backup` right after install.

## Verify

- `paseo plugin ls skill-pins --json` -> `running`, no load error.
- `paseo plugin logs skill-pins` -> no errors after the test turns.
- Manual in Paseo desktop (needs a human) -> the Claude agent timeline shows Skill tool calls for the pinned skills.
- `cat "$PASEO_HOME/plugin-data/skill-pins/agents/<id>/last-hook.json"` -> lists the pinned skills and contains no prompt text.
- Manual in Paseo desktop (needs a human) -> the Caveman menu still changes the reply style.
- `diff <(jq -S 'del(.hooks.UserPromptSubmit)' ~/.claude/settings.json.skill-pins-backup) <(jq -S 'del(.hooks.UserPromptSubmit)' ~/.claude/settings.json)` -> no output.
