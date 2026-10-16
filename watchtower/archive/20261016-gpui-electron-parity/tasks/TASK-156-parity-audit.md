# TASK-156 Side-by-side parity audit

Group: L (writes only the audit report)
Class: docs

## Brief

Goal: Record how close the integrated GPUI app is to the Electron app, surface by surface, so the owner sees what is exact and what is left.

Change: per-TASK parity lists -> one audit with side-by-side screenshots of every in-scope surface and a ranked list of mismatches.

Boundaries:

- Do not change code. Each mismatch becomes a report row with the crate, the element, the Electron value, and the GPUI value.
- Reference: the in-place Electron build on 6770, as [watchtower/CONTEXT.md](watchtower/CONTEXT.md) says. Use mock agents only.
- Surfaces: window chrome, sidebar tree and menus, tab bar and splits, agent pane (messages, steps, each tool kind, cards, find), composer (rest, text, image pill, slash menu, running), explorer (files, changes, file view, diff view), command center, settings (each section), terminal, and both themes.
- Out-of-scope surfaces (see Decisions in [watchtower/CONTEXT.md](watchtower/CONTEXT.md)) get one row each marked "out of scope", so the gap to full parity is visible.
- The VoiceOver pass from the previous plan still needs a human; list it as a manual check.

How:

- Walk each surface in both apps at the same window size and theme with `cua_repl`, and save the screenshot pairs under `packages/desktop-gpui/docs/parity/`.
- Write the report with one row per surface element and a ranked mismatch list.

Files:

- [packages/desktop-gpui/docs/parity-audit.md](packages/desktop-gpui/docs/parity-audit.md) (new report)
- [packages/desktop-gpui/docs/parity](packages/desktop-gpui/docs/parity) (new screenshot pairs, PNG)

Interfaces:

- Consumes: the bundle `target/bundles/Bachuc GPUI.app` built by `bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop` from TASK-155

Expected result:

- The report has a row for every surface above, in both themes, with match or mismatch.
- Each mismatch names the crate and the exact values.
- The out-of-scope surfaces are listed.

Anti-goal: no file outside `packages/desktop-gpui/docs` changes, and the screenshots stay under 20 MB in total; tripwire; read with `git status --short` and `du -sh packages/desktop-gpui/docs/parity` at completion.

## Verify

- `rg -c "^\| " packages/desktop-gpui/docs/parity-audit.md` -> at least one row per surface listed in Boundaries, for both themes
- `ls packages/desktop-gpui/docs/parity/*.png | wc -l` -> at least two images per surface
- Manual check (needs a human): the VoiceOver pass with Cmd+F5 over the sidebar, a timeline message, and the composer -> recorded in the report as pending or done
- Anti-goal: `git status --short -- . ':!packages/desktop-gpui/docs'` -> no change from this TASK, and `du -sh packages/desktop-gpui/docs/parity` -> at most 20M
