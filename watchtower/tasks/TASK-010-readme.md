# TASK-010 README for the overview and pill

Group: I (standalone: `README.md` of the plugin)
Class: docs

## Brief

Goal: Document the full-screen overview and the composer pill in the plugin README.

Change: the README covers the panel and attachments only -> it also covers the pill, the overview, and the new RPC.

Boundaries:

- Integrate into the existing sections. Do not append a loose paragraph at the end. See "Writing docs" in [CLAUDE.md](CLAUDE.md).
- Describe only what shipped. Read the TASK-001 to TASK-009 outcomes first.

How:

- In `## Use`, add how the pill opens the surface, and that wide layouts show the overview while compact ones keep the board.
- Name the six overview parts in one short list.
- In `## Format and limits`, add the files the overview reads (`RUN.md`, `QUESTIONS.md`, `DECISIONS.md`, and up to 5 archived plans) and the timeline rules.

Files:

- [hiep-plugins/plugins/watchtower-board/README.md](hiep-plugins/plugins/watchtower-board/README.md) (pill, overview, and limits)

Expected result:

- A reader learns how to open the overview, what each part shows, and which files feed it.

Anti-goal: the README stays short: added lines stay at or below 40; drift gauge; read with `git diff --numstat -- hiep-plugins/plugins/watchtower-board/README.md` at completion.

## Verify

- `rg -n "pill" hiep-plugins/plugins/watchtower-board/README.md` -> at least one match in `## Use`.
- `npm run format:check:files -- hiep-plugins/plugins/watchtower-board/README.md` from the repo root -> exit 0.
- `git diff --numstat -- hiep-plugins/plugins/watchtower-board/README.md` -> 40 added lines or fewer.
