# TASK-002 outcome

Status: DONE

## Evidence

- Baseline: `npm test` passed 65/65 before changes; `/tmp/recap-v2-baseline.log`.
- Source reproduction: web.ts only renders fenced prompt controls; no Recap adapter exists.
- Desktop baseline: actual fixture thread showed ordinary Recap bullets and one shared box for all three suggestions.
- Desktop acceptance agent: `b5662122-4541-46e9-b6e3-8a07cf3b112a`. Acceptance pending.

## Handoff checkpoint — 2026-09-25

- Changed: compact bullet-form Recap adapter, separated selection groups, shared count/action bar, direct actions for one structured suggestion.
- Local checks: format, typecheck, lint passed; tests 69/69 in `/tmp/recap-v2-tests.log` (baseline 65/65).
- Reload: `next-prompt-actions` running; logs show Plugin ready at 2026-09-25T11:24:10.381Z, no load error.
- Desktop NOT accepted: the latest fixture reply still showed ordinary Recap bullets and raw next-prompts JSON after reload. Root cause unconfirmed.
- Earlier desktop rendering of the old selection UI worked. Another user-driven combined send appeared in the fixture, but this does not verify the new layout or the parent's Edit/Send checks.
- CUA developer-tools attempt was interrupted by the user; no console diagnostic obtained. Reacquire live app state before continuing.
- Reuse fixture agent `b5662122-4541-46e9-b6e3-8a07cf3b112a`, workspace `wks_f556e297c968f167`. Archive fixture after final acceptance.
- Preserve unrelated concurrent changes under `plugins/prompt-translate/`.
- No commit, push, global policy change, Paseo source change, or daemon restart during this UI implementation.

## Handoff checkpoint 2 — 2026-09-25 11:36 UTC

- Desktop verification blocked: `cua_repl` `cua.getApp("Paseo")` rejects with `Computer Use was not approved to use Paseo` (app `sh.paseo.desktop` is running). Per AGENTS.md, no fallback tool used; desktop work stopped.
- No code changed in this checkpoint. Plugin still `running`; last reload 11:24:10Z.
- Root cause of raw JSON after reload still unconfirmed; needs live DOM/console inspection once Computer Use approval for Paseo is granted.

## Handoff checkpoint 3 — 2026-09-25 unified panel

- Changed: the prompt block now renders one complete panel. It holds the Recap strip, the What Next title, intro paragraphs, suggestions, and actions. See `foldPanel` in [recap.ts](../../plugins/next-prompt-actions/client/recap.ts).
- Native Recap, What Next heading, and intro nodes are hidden in place with `data-npa-folded`, never moved. Cleanup restores the message markup exactly.
- Local checks: typecheck, lint, format passed; tests 70/70 in `/tmp/recap-panel-tests.log`.
- Fixture evidence: Paseo-like DOM fixture in `artifacts/recap-panel/` (ignored). Screenshots `/tmp/recap-panel-single.png`, `/tmp/recap-panel-multi.png`, `/tmp/recap-panel-narrow-dark.png`.
- Plugin reloaded and running. Desktop rendering not yet verified; fixture evidence does not prove the live host DOM.

## Handoff checkpoint 4 — 2026-09-25 approved design port

- Ported the approved [panel design](../../docs/designs/recap-next-panel-2026-09-25/panel.html) into `client/web.ts`, `client/recap.ts`, and `client/selection.ts`.
- Panel sections: Recap (kicker, branch and Commit/push chips, Did text), What Next (title, intro, suggestions), selection footer. Colors derive from the host ink and background.
- Local checks: typecheck, lint, format passed; tests 70/70 in `/tmp/recap-panel-port-tests.log`.
- Fixture screenshots without a composer draft: `artifacts/recap-panel/np-port-*.png`. Plugin reloaded and running. Desktop rendering still unverified.

## Handoff checkpoint 5 — 2026-09-25 desktop fold fix

- Root cause: Paseo 0.9.1 renders each Markdown block of one reply as its own history row (`data-message-id`, `data-history-row-id="<id>:block:N"`), each with its own `assistant-message`. Fold and What Next heading marking searched only the prompt block's row, so desktop showed the native Recap and heading.
- Evidence: temporary diagnostic RPC captured the live DOM, saved as `artifacts/recap-panel/desktop-dom-2026-09-25.log`. The diagnostic code is removed.
- Fix: `messageParts` in [recap.ts](../../plugins/next-prompt-actions/client/recap.ts) collects rows sharing the message ID. Fully folded rows are hidden. Fold reapplies when the host re-renders a hidden node.
- Checks: regression test for split rows went RED then GREEN; tests 71/71 in `/tmp/recap-panel-rows-tests.log`; typecheck, lint, format passed. Replay of the captured desktop DOM folded Recap and What Next rows.
- Live desktop report after reload: folded `block:4` (Recap) and `block:5` (What Next), panel contained Recap and title. Visual review by the user still pending.

## Handoff checkpoint 6 — 2026-09-26 colors and icons

- Design: [panel-v2.html](../../docs/designs/recap-next-panel-2026-09-25/panel-v2.html) adds section colors and icons. Recap is blue (history badge, done icon, branch chip). What Next is amber (arrow badge, prompt icon, reason bulb, selected rows). Committed or pushed status is green.
- Ported into `client/web.ts`, `client/recap.ts` (badges), and `client/selection.ts` (group classes). Hues mix with the host ink, so dark mode follows the host.
- Checks: typecheck, lint, format passed; tests 71/71 in `/tmp/recap-panel-colors-tests.log`. Fixture screenshots `artifacts/recap-panel/np-colors-*.png`. Plugin reloaded and running. Desktop visual review pending.

## Handoff checkpoint 7 — 2026-09-26 premium v3 port

- Ported the approved [v3 design](../../docs/designs/recap-next-panel-2026-09-25/panel-v3.html) under [TASTE.md](../../docs/designs/recap-next-panel-2026-09-25/TASTE.md): neutral host-derived surfaces, one emerald accent for the What next area, radii 12/8/6, Phosphor regular icons (MIT) as CSS masks.
- Recap: recessed surface, sentence-case label. The commit chip shows "No commit" for `none`, keeps the raw value with an `aria-label`, and swaps the icon for committed or pushed state. The v2 hues, eyebrow, row icons, and group classes are removed.
- Theme: the client sets `data-npa-theme` from host ink brightness so the accent and shadow switch in dark mode.
- Checks: typecheck, lint, format passed; tests 71/71 in `/tmp/recap-panel-v3-tests.log`. Fixture screenshots `artifacts/recap-panel/np-v3-*.png`. Plugin reloaded and running. Desktop visual review pending.

## Handoff checkpoint 8 — 2026-09-26 section gap

- Live desktop box report showed a 6px flex gap between the Recap and What Next sections (for example Recap ends at 1305, What Next starts at 1311). The current sheet sets no gap; the committed older sheet set `gap:6px` on `[data-npa-block] > [owner]`. The source of that older rule in the page is unconfirmed; a second diagnostic run produced no report.
- Fix: `client/web.ts` pins `gap`, `margin`, and `padding` to 0 with `!important` on the panel container. Fixture check with the old rule injected after load measured a 0px gap.
- Checks: typecheck, lint, format passed; tests 71/71 in `/tmp/recap-panel-gap-tests.log`. Diagnostic code removed. Plugin reloaded and running.

## Handoff checkpoint 9 — 2026-09-26 read-only earlier replies

- Cause: the server snapshot only holds candidates for the current turn, so after a new send the client cleared earlier panels and the raw fence returned.
- Fix: `client/web.ts` parses an earlier reply's fence on the client and renders a read-only panel (`data-npa-readonly`) with Recap, What Next, and suggestion text, without buttons or inputs. It applies only when the nearest heading is What Next or Next Steps (`underNextHeading` in `client/recap.ts`).
- Checks: new regression test went RED then GREEN; tests 72/72 in `/tmp/recap-panel-readonly-tests.log`; typecheck, lint, format passed. Replay of the captured desktop DOM with no server candidates rendered read-only panels for all three earlier replies. Plugin reloaded and running.

## Handoff checkpoint 10 — 2026-09-26 scroll jank

- Verified cause on a virtualized-history fixture (`artifacts/recap-panel/virtual.html`): a newly mounted earlier reply painted raw Markdown for 11 frames (165 ms with a 40 ms read) before the panel replaced it. Paseo's TanStack virtualizer then re-measured the changed rows, which reads as jumping.
- Fix in `client/web.ts`: the mutation callback folds newly mounted rows synchronously, before paint, as read-only panels; the scan still upgrades the current reply. Late rows of the same reply trigger a refold; rows that unmount with their node do not. A failed timeline read keeps read-only panels instead of clearing them.
- After fix: the panel, folded Recap row, and Recap section exist in the first frame (1 frame, 5 ms). No rebuild churn over later scans (2 panel inserts for 2 replies).
- Checks: new regression test (read never settles) went RED then GREEN; tests 73/73 in `/tmp/recap-panel-virtual-tests.log`; typecheck, lint, format passed. Plugin reloaded and running. Live scroll feel still needs the user's check.

## Handoff checkpoint 11 — 2026-09-26 scan race

- New fixture `artifacts/recap-panel/tanstack.html` drives Paseo's own `@tanstack/virtual-core` 3.13.21 with Paseo's measurement pattern (absolute rows, rAF-deferred `measureElement`, scroll correction for rows above the offset, block-height estimate cache that ignores 0). It counts scroll corrections while scrolling 40 px per frame.
- Root cause of the remaining jitter: the async scan listed assistant rows before awaiting the timeline read, then cleared panels, Recap decorations, and heading marks that `paintNow` created for rows mounted during the await. Rows unfolded and refolded, so the virtualizer corrected the scroll (12 corrections per warm pass).
- Fix in `client/web.ts`: scan cleanup only settles nodes it queried or nodes that left the page.
- Tried and reverted: collapsing folded rows to a 1px body and lifting the panel over them. It measured no better than `display:none` and added layout risk.
- Result: warm upward passes 0 corrections with and without the plugin; cold passes 97 vs 95 (Paseo's native first measurement).
- Checks: race regression test went RED then GREEN; tests 74/74 in `/tmp/recap-panel-race-tests.log`; typecheck, lint, format passed. Plugin reloaded and running. Live scroll feel still needs the user's check.

## Desktop evidence — 2026-09-26 (user-reported)

- After the scan race fix, the user scrolled Paseo desktop over earlier replies: first pass smooth, second pass smooth.
- Earlier user screenshots confirmed on desktop: the folded Recap and What Next panel, read-only panels on earlier replies, and a working Send from the panel.
- Still unconfirmed on desktop: exclusive choices and allowed combinations in a multi-choice reply, and Edit filling the composer.

## Multi-choice fixture — 2026-09-26

- The old fixture agent `b5662122-...` and its workspace were already archived.
- New disposable fixture agent `9dd06a77-8b3d-471a-9bac-a1b48377a439` ("Multi-choice panel fixture", workspace `wks_134d55bd1366ab3a`) replied once with Recap, What Next, and a `next-prompts` block: exclusive group `brief`/`detailed`, extra `checklist`, combinations `brief+checklist` and `detailed+checklist`.
- The real parser accepts the block. Selection rules: `brief` alone, `brief+checklist`, `detailed+checklist` allowed; `brief+detailed` and all three rejected.
- Desktop clicks not performed: `cua_repl` still fails with `Computer Use was not approved to use Paseo`. Needs the user's manual check; archive the fixture agent afterward.
- README section "Recap and next-step layout" updated for the v3 panel and read-only earlier replies.

## Acceptance — 2026-09-26

- The user reported TASK-002 complete after checking the multi-choice fixture on desktop.
- Code shipped in commit `a0b286a` on `main`. The disposable fixture agent `9dd06a77-8b3d-471a-9bac-a1b48377a439` is archived.
