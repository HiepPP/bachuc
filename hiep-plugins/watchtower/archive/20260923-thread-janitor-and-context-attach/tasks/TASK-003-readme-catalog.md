# TASK-003 Root README catalog entries

Group: C (writes only the root `README.md`)
Class: docs

## Brief

Goal: List both new plugins in the root README catalog.

Change: README lists nine plugins -> README lists eleven plugins.

How:

- Add one short section per plugin, in the same style as the existing sections.
- Link each section to the plugin README.
- Update the plugin count in the intro and add both IDs to the Pick your plugins list.
- Do not add screenshots unless a real capture exists.

Files:

- [README.md](README.md) (two new catalog sections, count, and plugin list)

Expected result:

- The root README links to [plugins/thread-janitor/README.md](plugins/thread-janitor/README.md) and [plugins/thread-context-attach/README.md](plugins/thread-context-attach/README.md).

## Verify

- `rg -n "thread-janitor/README.md|thread-context-attach/README.md" README.md` -> one link for each plugin.
- `rg -n "Eleven independent" README.md` -> one match.
