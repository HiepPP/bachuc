# Learn 20261017-gpui-daily-gaps-and-fixes

## Summary

Discrepancy: 27 found. All 34 TASKs are DONE and all Tracker commits are on the branch, but many checks need the pointer, and 6 owner checks stay open (Q-142).

## Per TASK

- TASK-198: plan was a shell-only fix with a latest-prompt reveal -> shipped after 2 fix rounds that also changed the timeline and workspace crates (Q-120). Mistake: the Brief missed subscription order, tail follow in `scroll_to_reveal_item`, and the missing AX Click on tab chips. Fix: before you plan a reveal or AX check, read the view sync order and the list scroll code.
- TASK-199: plan was "the agent tab shows an empty pane" -> the real bug was "the Board stays in front after a sidebar click", fixed by `150a36be3` (Q-127). Mistake: the Brief guessed the bug and did not reproduce it first. Fix: get the owner's exact steps before you write a bug TASK.
- TASK-200: plan said an archived agent arrives as an upsert with `archivedAt` -> the daemon sends `remove`, so a fix round added `fetch_agent` (`aaada0a61`). Mistake: the plan trusted a protocol shape and did not read the server. Fix: read the server filter code for each event a client TASK depends on.
- TASK-201: match.
- TASK-202: plan had header buttons and a toast host -> shipped, but the anti-goal had no "before" read and the 900 px check needed the pointer. Mistake: the plan put the toast host late, so TASK-211/212/213 used inline errors that TASK-222 later replaced. Fix: build shared UI hosts first.
- TASK-203: match.
- TASK-204: match.
- TASK-205: plan routed archived upserts into the archived map -> correct as coded, but the daemon never sends such an upsert to this client. Mistake: wrong protocol premise (see TASK-200). Fix: test with what the daemon really sends to that subscription filter.
- TASK-206: match.
- TASK-207: plan said "keep the 24 px pitch, it matches Electron" -> a measure showed 24 pt vs 21 pt, and a fix round changed it. The quote style and letter spacing were not possible (Q-117). Mistake: the Brief trusted an old code comment. Fix: measure before you state a pitch, and check vendor APIs before you plan a style.
- TASK-208: plan made a fork boundary with a cursor -> shipped the message id only, because the timeline epoch was private (Q-129). The fix came in TASK-220 and TASK-221. Mistake: an interface gap in the client. Fix: check that each consumed value is public before you plan it.
- TASK-209: plan said the numbers match and the fault is at render time -> a fix round fixed a star past the card edge (negative margin in Taffy) and AX presses. The Electron reference Board did not load. Mistake: no reference was ready for the check. Fix: confirm the reference app loads the plugin before UI checks.
- TASK-210: plan disabled every action while any action runs -> shipped as specified, but Electron disables only the running action (Q-121). Mistake: spec gap against Electron. Fix: copy the Electron enable rule exactly.
- TASK-211: plan had a branch switcher -> shipped with inline errors, an "Unknown" label for a detached HEAD, and no arrow keys (Q-122). TASK-222 and TASK-223 reworked it. Mistake: no toast host yet, and the parity list was incomplete. Fix: list Electron behaviors in full in the Brief.
- TASK-212: plan had a PR view -> shipped with no check timing, no ring, and plain bodies (Q-124). No real PR was available to check it. Mistake: the client lacked the fields, and there was no test repo with a PR. Fix: add the client fields first, and prepare a PR fixture.
- TASK-213: plan had a file menu with UI checks -> the menu opens only by right-click, so every check needed the owner. TASK-225 added the missing items (Q-132). Mistake: the plan did not know accesskit maps no show-menu action. Fix: give each menu an AX or keyboard path.
- TASK-214: match.
- TASK-215: plan had a flood timing anti-goal -> it was not run, because no baseline bundle existed. TASK-227 reworked the gaps (Q-134). Mistake: the plan did not prepare a baseline. Fix: build the "before" bundle before the TASK starts.
- TASK-216: plan put Search before History, and Files had no `lib.rs` -> shipped in Electron order with a `pub use` in `lib.rs` (Q-126). Mistake: the spec order was wrong and the Files list missed the re-export. Fix: check the Electron order and the crate exports in the Brief.
- TASK-217: plan had a Files scope -> shipped with no error line and no Files section in the full list (Q-135). TASK-228 reworked it. A doctest failed once, then passed. Mistake: incomplete parity list. Fix: list empty, error, and full-list states for each scope.
- TASK-218: plan had a build-time anti-goal -> it could not run, because a clean build needs `cargo clean`, and builders may not run it. Mistake: the anti-goal conflicted with builder rules. Fix: write anti-goals the builder can measure.
- TASK-219: plan Files had no composer `lib.rs` -> the builder added a `DraftSeed` export there. Mistake: the Files list missed the re-export. Fix: add `lib.rs` when a new type crosses crates.
- TASK-220: plan added public fields -> the new fields broke the tests of `bachuc-explorer` and `bachuc-app` until TASK-222 and TASK-231 ran. Mistake: a dep gap; struct literals in other crates. Fix: update dependent literals in the same TASK, or use a constructor.
- TASK-221: match.
- TASK-222: plan had two menu gates -> shipped, but the gate checks need the pointer and an old host. Mistake: no AX path and no old-host fixture. Fix: test gates in pure tests, and leave the screen proof to the owner from the start.
- TASK-223: plan Files listed only `branch_switcher.rs` -> the "Unknown" label was in `explorer.rs`, so the builder edited it too. Mistake: wrong file target. Fix: grep the UI string before you list Files.
- TASK-224: plan had a ring, durations, and Markdown -> shipped, but no live PR existed for the check (Q-142). The timeline Markdown style could not be reused (crate cycle). Mistake: no PR fixture. Fix: keep a scratch repo with an open PR for UI checks.
- TASK-225: plan had Electron file menu items -> shipped, but the right-click checks are open (Q-142), and Reveal shows on every folder row (Q-140). Mistake: no local-host flag and no AX menu path. Fix: same as TASK-213.
- TASK-226: match.
- TASK-227: plan had wrapped links and a kept query -> shipped, but Cmd-click and flood timing were not run (Q-142). Mistake: pointer-only check. Fix: add an AX action for links, or plan an owner check.
- TASK-228: plan had a muted error line in the Files section -> shipped Electron's "Error: <message>" line (Q-137). Mistake: spec gap against Electron. Fix: copy the Electron text and place it from source.
- TASK-229: plan had page state focus handles -> shipped in keyed element state (Q-141), after 2 fix rounds: no tab stops, then a filled shadow ring and `in_focus` misuse. Off-screen cards are not reachable (Q-143). Mistake: the Brief did not know GPUI focus rules and missed `runs.rs`. Fix: prototype GPUI focus behavior before you plan, and include the list file.
- TASK-230: plan had an overlay with an owner-only check -> shipped, but the owner check is open (Q-142). Mistake: deferred verify by design. Fix: run the owner check before the archive.
- TASK-231: plan had Electron's Add to chat flow -> shipped without the parent-tab step (Q-139), and Add to chat is an open owner check (Q-142). Mistake: GPUI tabs record no parent. Fix: plan a parent-tab field if parity is needed.

## Plan-Level

- Scope grew in the middle: commit `577301704` added TASK-220 to TASK-231. They reverse "keep them for this plan" answers (Q-116, Q-122, Q-124, Q-128 to Q-130, Q-132, Q-134, Q-135).
- The plan misdescribed reality twice: TASK-199 was re-scoped by Q-127, and TASK-207 trusted a wrong pitch comment.
- Wrong order: the toast host came in TASK-202, after the views that needed it. TASK-222 then reworked TASK-211, TASK-212, and TASK-213.
- Dep gap: TASK-208 needed the timeline epoch, which TASK-220 added later. TASK-220 broke dependent crate tests until TASK-222 and TASK-231.
- Files lists missed files 5 times: TASK-216, TASK-219 (`lib.rs` exports), TASK-223 (`explorer.rs`), TASK-229 (`runs.rs`), TASK-198 (timeline and workspace crates).
- Fix rounds: TASK-198 (2), TASK-229 (2), TASK-200 (1), TASK-207 (1), TASK-209 (1).
- Verify gaps: the background checker cannot use the pointer, so many checks moved to the owner (Q-133, Q-142).
- Timing anti-goals could not run: the app logs no click or paint timestamps (TASK-198, TASK-200, TASK-202, TASK-217). Builders took no "before" reads (TASK-202, TASK-211, TASK-213, TASK-215).
- Commit `77a9730cb` mixes the TASK-207 fix round and TASK-208. One commit per TASK keeps the Tracker clear.
- Shared checkout: parallel builders broke each other's builds once (TASK-226 notes this).
- NEXT.md Handoff says "All 22 TASKs are DONE", but the Tracker has 34 TASKs. Update the line at archive.
- Status is IMPLEMENTED, but Q-142 is still OPEN.

## Lessons

- Reproduce a bug before you write its TASK, and record the owner's exact steps.
- Read the server send path before you plan a client reaction to an event.
- Measure Electron before you plan a visual change. Do not trust code comments.
- Build shared hosts (toast, notices) before the views that use them.
- When a new type crosses crates, add the `lib.rs` export to Files.
- When a TASK adds struct fields, update every literal in other crates in the same TASK.
- In `gpui-pre`, `.focusable()` is not a tab stop, and a spread `BoxShadow` is a filled quad.
- Do not put a negative main-axis margin on a child of a content-sized GPUI flex row.
- Every control an AX check presses needs an `AccessibleAction::Click` handler.
- Prepare fixtures early: a repo with a real PR, an old-host daemon, and a baseline bundle.
- Add click and paint log timestamps, so timing anti-goals can run.
- Plan pointer-only checks as owner checks from the start.

## Open Checks For Owner

- Q-142: file menu without Discard on an old host, and without Add to chat with no chat tab (TASK-222).
- Q-142: Add to chat chip on an agent tab and on a draft tab (TASK-231).
- Q-142: right-click a changed file (Duplicate) and a changed folder (TASK-225).
- Q-142: a real PR with the checks ring, durations, and Markdown (TASK-224).
- Q-142: Cmd-click a wrapped terminal path (TASK-227).
- Q-142: drag a file over the composer and away, then drop (TASK-230).
- TASK-229: seen card un-dim on focus was not checked in run E (all cards were new); run D covers it.
- Timing anti-goals (click to paint, Cmd+K, flood) stay UNVERIFIED.

## Open ADR Candidates

- Fetch an unknown agent on demand instead of subscribing with `includeArchived` (TASK-200).
- Keep archived agents in a separate client map, not in the live directory (TASK-205).
- One app toast host at the top center for all explorer notices; no inline errors (TASK-202, TASK-222, TASK-226).
- Board card focus handles in GPUI keyed element state, not page state (TASK-229, Q-141).
- Show a home path as `~/...` in Edit rows, unlike Electron (TASK-207, Q-118).
- Keep the vendored quote style and no letter spacing until a gpui-base patch (Q-117, ADR-0044).
- Bind bare-key shortcuts such as `?` with a `!` context predicate to keep them out of text fields (TASK-203).
