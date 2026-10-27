# TASK-319 Investigation answer rule

Group: D (the global agent rules file outside the repo; standalone)
Class: docs

## Brief

Goal: An agent that answers a bug hunt or an investigation writes Symptom, Cause, and Fix, and cites code with the citation fence that TASK-318 draws as a card.

Change: the Output section has a generic investigation order -> it also gives the Symptom / Cause / Fix shape and the citation fence format.

Boundaries:

- The file is `~/.claude/CLAUDE.md`, outside the repo. It is the shared global policy for Claude, Codex, and OpenCode through links. Edit it in place; do not break the links named at its top.
- It is not part of the plan commit. The lead edits it and records the diff in the outcome sidecar.
- Keep the existing rule "answer, then What I did, What happened, Why it matters". Add the new shape for root-cause answers only, in at most 4 bullets.
- The fence format must match Q-197: ` ```<lang> <path>:<start>-<end> `, with a short excerpt of at most 15 lines.

How:

- Add the bullets in the `## Output` section, next to the investigation order rule.

Files:

- `~/.claude/CLAUDE.md` (Output section; outside the repo, so no markdown link)

Expected result:

- WHEN an agent reads the Output section THE rules SHALL tell it to answer a root-cause question with Symptom, Cause, and Fix, and to cite code with the citation fence.

Anti-goal: the Output section grows by at most 4 bullets and no existing bullet changes meaning; tripwire; read with `diff` against a copy taken before the edit, at completion.

## Verify

- `cp ~/.claude/CLAUDE.md /tmp/claude-md-before.md` before the edit, then `diff /tmp/claude-md-before.md ~/.claude/CLAUDE.md` -> only added lines, at most 4, all in `## Output` (anti-goal).
- `ls -l ~/.claude/AGENTS.md ~/.codex/AGENTS.md ~/.config/opencode/AGENTS.md` -> the links still point where they pointed before.
