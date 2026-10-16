# TASK-156 Outcome

## Status

DONE. The audit is written from existing UI check evidence; no new screenshots were taken.

## Changed

- `packages/desktop-gpui/docs/parity-audit.md` (new, 145 table rows, one H1)
- `packages/desktop-gpui/docs/parity/` (43 PNG files, 14 MB: 21 gpui/electron pairs and one GPUI-only split capture)

## Verified

- `rg -c "^\| " packages/desktop-gpui/docs/parity-audit.md` -> PASS: 145 rows; every surface has rows for both themes (dark rows are UNVERIFIED where no reference exists).
- `ls packages/desktop-gpui/docs/parity/*.png | wc -l` -> PASS: 43; two images per surface (splits has the GPUI image only, the reference was never captured).
- VoiceOver pass -> recorded as pending (needs a human).
- `npm run format:files -- packages/desktop-gpui/docs/parity-audit.md` -> PASS.

## Anti-goal

`git status --short -- . ':!packages/desktop-gpui/docs'` shows no change from this TASK except this sidecar. `du -sh packages/desktop-gpui/docs/parity` -> 14M (limit 20M).

## Lessons

- A background GPUI window does not draw (Q-069), so a new capture needs the foreground. The latest saved pairs under `/tmp/gpui-ui-check/` are enough for the light theme; dark pairs exist only for the sidebar and the Explorer Files tab.
- Open owner questions Q-086 and Q-088 stay in the report.
