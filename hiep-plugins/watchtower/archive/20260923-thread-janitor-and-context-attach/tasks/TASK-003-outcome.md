# TASK-003 Root README catalog entries

Status: DONE

Changed:
- README.md: bumped intro count to "Eleven independent", added a section each for
  Thread janitor and Thread context attach (linking to their plugin READMEs), and
  added both to the "Pick your plugins" table.

Contract:
- README links to plugins/thread-janitor/README.md and plugins/thread-context-attach/README.md.
- Docs-only change; no code/schema contract affected.

Verified:
- `rg -n "thread-janitor/README.md|thread-context-attach/README.md" README.md` -> matches at lines 138, 146, 165, 166 (one link per plugin in body plus table row).
- `rg -n "Eleven independent" README.md` -> one match (line 6).
