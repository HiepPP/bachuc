# TASK-003 Outcome

## Outcome

Status: DONE

Changed:
- [plugins/thread-context-attach/skills/paseo-thread-search/SKILL.md](plugins/thread-context-attach/skills/paseo-thread-search/SKILL.md): new skill.
- [plugins/thread-context-attach/README.md](plugins/thread-context-attach/README.md): new `## Search from agents` section, and the intro mentions search.
- New symlinks outside the repo: `~/.claude/skills/paseo-thread-search` and `~/.codex/skills/paseo-thread-search`, both pointing to the repo skill folder.

Contract:
- The skill tells agents to search with 2 or 3 keywords, retry with synonyms, then use `qmd query`, and to read only about 80 lines around a hit.
- The skill tells agents to cite `thread <id8>, Turn N`, to skip hits from the current conversation, and not to guess when nothing is found.

Verified:
- `ls -l` on both links -> both are symlinks to the repo skill folder.
- `qmd --index paseo-threads search "watchtower sidebar" -n 5` -> exit 0.
- `qmd --index paseo-threads query "why does the git pill disappear" -n 5` -> exit 0 in 8 seconds. The first hit is `5522b6d9`, a thread about the git pill.
- `qmd --index paseo-threads get "qmd://paseo-threads/241e4fc4-...md:16" -l 80` -> exit 0.
- `rg -n "^## Turn" <file>` -> lists turn headings with line numbers.
- `npx oxfmt --check README.md` -> exit 0.
- This Claude session listed `paseo-thread-search` among its skills right after the link was made.
- PENDING-USER: ask a new agent in the Paseo app to find what other threads decided about the watchtower sidebar, and check that it uses the skill.
