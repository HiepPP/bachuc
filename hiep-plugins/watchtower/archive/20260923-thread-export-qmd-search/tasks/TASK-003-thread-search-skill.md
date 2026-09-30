# TASK-003 Skill that tells agents to search threads

Group: B (skill folder and README.md)
Class: docs

## Brief

Goal: Claude and Codex agents know when and how to search other Paseo threads with qmd, and they read only the matching turn.

Change: agents do not know thread files exist -> a `paseo-thread-search` skill tells them the exact commands.

How:

- Add the skill source at `plugins/thread-context-attach/skills/paseo-thread-search/SKILL.md`. Follow the style of `~/.claude/skills/kb-search/SKILL.md`.
- Description: use when you need a decision, result, root cause, or other context from another Paseo thread or agent conversation. Do not use for code search or for the knowledge base.
- Steps in the skill:
  - Search with 2 or 3 distinctive keywords: `qmd --index paseo-threads search "<keywords>" -n 5`. Every word must appear in the file, so do not write a full sentence.
  - If keywords miss, retry with synonyms. Then try `qmd --index paseo-threads query "<question>" -n 5`. It is smarter but takes 2 to 30 seconds.
  - Read only the hit: `qmd --index paseo-threads get <path>:<line> -l 80`. Do not read a whole thread file.
  - Each hit shows `Thread: <agentId>` near the top of its file and a `## Turn N` heading. Cite the thread id and turn in the answer.
  - Results can include the current thread. Skip hits that repeat the current conversation.
  - If there is no hit, try 2 other keyword sets, then say that nothing was found. Do not guess.
- Confirm the real `get` path format with one live `search` before you write the example. Use the format qmd prints.
- Link the skill with symlinks: `~/.claude/skills/paseo-thread-search` and `~/.codex/skills/paseo-thread-search` point to the repo folder. Do not overwrite an existing path with that name.
- Add a `## Search from agents` section to [plugins/thread-context-attach/README.md](plugins/thread-context-attach/README.md). Cover the export folder, the file format, the qmd index, the skill links, privacy (files stay local, mode `0600`), and how to stop (disable the plugin).

Files:

- [plugins/thread-context-attach/skills/paseo-thread-search/SKILL.md](plugins/thread-context-attach/skills/paseo-thread-search/SKILL.md) (new skill)
- [plugins/thread-context-attach/README.md](plugins/thread-context-attach/README.md) (new section)
- `~/.claude/skills/paseo-thread-search` and `~/.codex/skills/paseo-thread-search` (new symlinks, outside the repo)

Expected result:

- A new Claude or Codex agent lists `paseo-thread-search` among its skills.
- The commands in the skill work as written.

## Verify

- `ls -l ~/.claude/skills/paseo-thread-search ~/.codex/skills/paseo-thread-search` -> both are symlinks to the repo skill folder.
- Run each command in the skill once -> each exits 0 and prints results in the documented format.
- `cd plugins/thread-context-attach && npx oxfmt --check README.md` -> exit 0.
- Manual, needs the Paseo app: in a new agent, ask "find in other threads what we decided about the watchtower sidebar". The agent uses the skill and cites the thread `241e4fc4` and a turn number.
