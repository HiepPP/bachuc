# GPUI parity audit

Date: 2026-10-17. Branch `feat/bachuc-gpui`. Reference: the in-place Electron build on 6770. Mock agents only; release (6767) was not touched.

## How to read this report

- Values come from the latest UI check of each element (TASK-147 to TASK-155). A re-check replaces an earlier failure. The code of TASK-168 to TASK-191 (composer tracks, New workspace screen, plugin clients) passed code tests only; its UI checks are deferred, so those rows are `UNVERIFIED`.
- This audit took no new screenshots. The existing pairs in `packages/desktop-gpui/docs/parity/` cover every surface that has a reference. A background GPUI window does not draw (Q-069), and the owner forbids bringing a tested app to the front. A surface with no pair is `UNVERIFIED`.
- Dark theme: only the sidebar and the Explorer Files tab have a dark pair. Other dark rows are `UNVERIFIED (no reference capture)`.
- Pair names are `<surface>-<theme>-gpui.png` and `<surface>-<theme>-electron.png`.
- Result: match (within 2 px), MISMATCH, known gap (recorded, with a Q-ID), or UNVERIFIED.

## Window chrome

| Element                    | Theme | Electron                      | GPUI                                  | Result                    | Crate | Pair           |
| -------------------------- | ----- | ----------------------------- | ------------------------------------- | ------------------------- | ----- | -------------- |
| Window, sidebar width      | light | 1200 x 800, sidebar 320       | 1200 x 800, sidebar 320               | match                     | app   | `window-light` |
| Sidebar and header fill    | light | `(244,244,246)`, header 36 px | `(244,244,246)`, header 36 px         | match                     | app   | `window-light` |
| Header right buttons       | light | VS Code split, Commit split   | `...` menu and panel toggle           | MISMATCH                  | app   | `window-light` |
| Theme switch, whole window | both  | -                             | one frame, no light region left       | match (GPUI only)         | app   | none           |
| 900 px window              | light | -                             | sidebar 320, Explorer 240, no overlap | match (GPUI only)         | app   | none           |
| Closed-port host           | light | -                             | "offline" state, Retry, Manage host   | match (GPUI only)         | app   | none           |
| Dark window as a whole     | dark  | not captured                  | not paired                            | UNVERIFIED (no reference) | app   | none           |

## Sidebar tree and menus

| Element                      | Theme | Electron                          | GPUI                             | Result                        | Crate   | Pair                            |
| ---------------------------- | ----- | --------------------------------- | -------------------------------- | ----------------------------- | ------- | ------------------------------- |
| Width, background            | light | 320, `#f4f4f6`                    | 320, `#f4f4f6`                   | match                         | sidebar | `sidebar-light`                 |
| Width, background            | dark  | `(26,25,23)`, border `(43,42,40)` | same                             | match                         | sidebar | `sidebar-dark`                  |
| Workspace row pitch, title   | both  | 38 px, title at x 40              | 38 px, title at x 40             | match                         | sidebar | `sidebar-light`, `sidebar-dark` |
| Project row to first row     | light | 42 px                             | 40 px                            | match                         | sidebar | `sidebar-light`                 |
| Diff counts on rows          | both  | `+55 -35` green and red           | same, from uncommitted changes   | match (source differs, Q-078) | sidebar | `sidebar-light`                 |
| Project order                | light | stored order, `gpui-bench` last   | alphabetical, `gpui-bench` third | MISMATCH (Q-085)              | sidebar | `sidebar-light`                 |
| Project letter-box color     | light | `.claude` brown, eva blue         | `.claude` indigo, eva purple     | MISMATCH (Q-077)              | sidebar | `sidebar-light`                 |
| Kebab menu by click and AX   | light | menu opens                        | Copy path, Rename, Pin, Archive  | match                         | sidebar | none                            |
| Rename, pin, archive on live | -     | -                                 | not run                          | UNVERIFIED (live down)        | sidebar | none                            |
| Navigation rows, heading     | light | New workspace, History, Schedules | Search only                      | out of scope                  | sidebar | `sidebar-light`                 |
| PR and checks line           | light | `83733 Merged · passed`           | none                             | out of scope (PR panel)       | sidebar | `sidebar-light`                 |

## Tab bar and splits

| Element                         | Theme | Electron                | GPUI                                | Result                     | Crate     | Pair                |
| ------------------------------- | ----- | ----------------------- | ----------------------------------- | -------------------------- | --------- | ------------------- |
| Bar height with border          | light | 36 px, border `#e4e4e6` | 35 px, border `#e4e4e6`             | match (1 px)               | workspace | `tabbar-light`      |
| Chip height, fill               | light | 28 px, `#f4f4f6`        | 28 px, `#f4f4f6`                    | match                      | workspace | `tabbar-light`      |
| Chip top offset                 | light | 4 px                    | 2 px                                | match within 2 px          | workspace | `tabbar-light`      |
| Icon and label offset           | light | 9 to 10 px, 26 px       | 9 px, 26 px                         | match                      | workspace | `tabbar-light`      |
| Close X, plus, ellipsis         | light | listed places           | same places                         | match                      | workspace | `tabbar-light`      |
| Inactive tab color, chip width  | light | one tab only            | -                                   | UNVERIFIED (no pair)       | workspace | none                |
| Tooltip, context menu, maximize | light | present                 | not built                           | MISMATCH (known gap)       | workspace | `tabbar-light`      |
| Tab icon for an agent           | light | Claude sparkle          | bot icon                            | MISMATCH                   | app       | `window-light`      |
| Split right and down, dividers  | light | not captured            | two and three groups, dividers move | match (GPUI only)          | workspace | `splits-light-gpui` |
| Divider grab band               | light | -                       | 1 to 2 px off the drawn line        | MISMATCH                   | workspace | none                |
| Drag a tab between groups       | light | -                       | tab joins the other group           | UNVERIFIED (needs pointer) | workspace | none                |
| Dark tab bar                    | dark  | not captured            | not paired                          | UNVERIFIED (no reference)  | workspace | none                |

## Agent pane

| Element                                          | Theme | Electron                           | GPUI                          | Result                          | Crate    | Pair                |
| ------------------------------------------------ | ----- | ---------------------------------- | ----------------------------- | ------------------------------- | -------- | ------------------- |
| Timeline anchor                                  | light | top: bubble at y 120 to 172        | y 120 to 172                  | match                           | app      | `agent-pane-light`  |
| Bubble top, paragraph pitch                      | light | 48, 47                             | 48, 48                        | match                           | timeline | `agent-pane-light`  |
| Heading and rule width                           | light | "Next Steps" 123 px, 1 px rule     | 122 px, 1 px rule             | match                           | timeline | `agent-pane-light`  |
| Heading rule to code block                       | light | 37 px                              | 25 to 26 px                   | MISMATCH (Q-086)                | timeline | `agent-code-light`  |
| Code block, one line                             | light | 43 px, `fn` magenta, `main` blue   | 43 px, same colors            | match                           | timeline | `agent-code-light`  |
| Fence label                                      | light | none                               | none                          | match                           | timeline | `agent-code-light`  |
| Turn header, step strip, rows                    | light | strip 169 px, header to step 39 px | 171 px, 41 px                 | match                           | timeline | `agent-steps-light` |
| Turn footer                                      | light | copy, fork, TTFT parts             | copy and "Worked for N"       | out of scope (fork, rewind)     | timeline | `agent-pane-light`  |
| Find, Cmd+F                                      | light | bar 342 px, 2 px focus ring        | bar 338 px, no focus ring     | MISMATCH (focus ring)           | timeline | `agent-find-light`  |
| File link click                                  | light | opens the file                     | `OpenFile` with path and line | match                           | timeline | none                |
| Image render, lightbox                           | light | 478 x 62                           | 478 x 62, lightbox opens      | match                           | timeline | none                |
| Image retry state                                | light | -                                  | not run                       | UNVERIFIED (needs second reply) | timeline | none                |
| Shell tool row expand                            | light | opens details                      | opens, no panic (3 of 3)      | match                           | timeline | none                |
| Pick step, then expand                           | light | -                                  | run before the pointer rule   | UNVERIFIED (needs pointer)      | timeline | none                |
| Read, edit, write, search, fetch, sub-agent rows | light | -                                  | no reference agent holds them | UNVERIFIED (live down)          | timeline | none                |
| Plan card, question card                         | light | -                                  | no agent holds them           | UNVERIFIED (live down)          | timeline | none                |
| Live answer or reject                            | -     | -                                  | not run                       | UNVERIFIED (live down)          | timeline | none                |
| Dark agent pane                                  | dark  | not captured                       | not paired                    | UNVERIFIED (no reference)       | timeline | none                |

## Composer

| Element                               | Theme | Electron                          | GPUI                          | Result                         | Crate    | Pair                   |
| ------------------------------------- | ----- | --------------------------------- | ----------------------------- | ------------------------------ | -------- | ---------------------- |
| Card width and inset                  | light | x 350 to 1170 in the 880 px pane  | x 350 to 1170                 | match                          | composer | `composer-light`       |
| Placeholder text, size, color         | light | 12 px, light grey, x 12 from edge | same                          | match                          | composer | `composer-light`       |
| Model pill, Claude sparkle            | light | sparkle                           | sparkle                       | match                          | composer | `composer-light`       |
| Pill chevrons                         | light | down chevron after each label     | down chevron after each label | match                          | composer | `composer-light`       |
| Focus hint                            | light | `⌘L to focus`, 13 px from edge    | 14 px from edge               | match                          | composer | `composer-light`       |
| Slash menu order, anchor              | light | `/exit` selected at the bottom    | same                          | match                          | composer | `composer-slash-light` |
| Slash detail card                     | light | 61 px high, 6 px gap              | 60 px high, 5 px gap          | match                          | composer | `composer-slash-light` |
| Slash row                             | light | 36 px                             | 36 px                         | match                          | composer | `composer-slash-light` |
| Mode pill menu                        | light | 5 modes, check on current         | 5 modes, check on current     | match                          | composer | none                   |
| Send button                           | light | green round                       | green round                   | match                          | composer | none                   |
| Image pill                            | light | -                                 | not run                       | UNVERIFIED (needs pointer)     | composer | none                   |
| Running state (Stop)                  | -     | -                                 | needs a running mock agent    | UNVERIFIED (live down)         | composer | none                   |
| Send, steer, queue, cancel            | -     | -                                 | not run                       | UNVERIFIED (live down)         | composer | none                   |
| Lightning toggle                      | light | present                           | built as a `fast_mode` toggle | UNVERIFIED (UI check deferred) | composer | `composer-light`       |
| Diff-count pill above input           | light | `+742 -0`                         | built (`diff_stat_pill.rs`)   | UNVERIFIED (UI check deferred) | composer | `composer-light`       |
| Track strip (tasks, subagents, queue) | light | present                           | built (`tracks.rs`)           | UNVERIFIED (UI check deferred) | composer | none                   |
| Plugin pills, stop orb                | light | present                           | built (`plugin_slots.rs`)     | UNVERIFIED (UI check deferred) | composer | none                   |
| @file mentions                        | light | present                           | none                          | MISMATCH (Q-080)               | composer | none                   |
| Microphone, waveform                  | light | present                           | none                          | out of scope (dictation)       | composer | `composer-light`       |
| Dark composer                         | dark  | not captured                      | not paired                    | UNVERIFIED (no reference)      | composer | none                   |

## New workspace and plugins

| Element                                   | Theme | Electron                  | GPUI                       | Result                         | Crate | Pair |
| ----------------------------------------- | ----- | ------------------------- | -------------------------- | ------------------------------ | ----- | ---- |
| New workspace screen, draft composer      | light | title, chip row, composer | built (`new_workspace.rs`) | UNVERIFIED (UI check deferred) | app   | none |
| Plugin surfaces (15 native clients)       | light | React clients             | linked in `plugins.rs`     | UNVERIFIED (UI check deferred) | app   | none |
| Plugin rows, settings screens, live reads | -     | -                         | not run                    | UNVERIFIED (live down)         | app   | none |

## Explorer

| Element                     | Theme | Electron                                    | GPUI                                                        | Result                      | Crate    | Pair                                          |
| --------------------------- | ----- | ------------------------------------------- | ----------------------------------------------------------- | --------------------------- | -------- | --------------------------------------------- |
| Tab rail, chip fill         | light | 36 px                                       | 36 px                                                       | match                       | explorer | `explorer-files-light`                        |
| Files row pitch, icons      | both  | 29 px                                       | 29 px                                                       | match                       | explorer | `explorer-files-light`, `explorer-files-dark` |
| Files toolbar               | both  | sort, new file, new folder, hidden, refresh | none                                                        | MISMATCH (follow-up needed) | explorer | `explorer-files-dark`                         |
| Changes header              | light | branch and `Uncommitted +6 -0`, 36 px rows  | same, passive labels, no menu                               | match (menus missing)       | explorer | `explorer-changes-light`                      |
| Changes row pitch, counts   | both  | 32 px, green and red                        | 32 px, same colors                                          | match                       | explorer | `explorer-changes-light`                      |
| Changes footer              | both  | `Commits` section                           | none                                                        | MISMATCH (follow-up needed) | explorer | `explorer-changes-light`                      |
| New-file mark               | both  | green "added" on `extra.rs`                 | modified mark on every row                                  | MISMATCH (Q-081)            | explorer | `explorer-changes-light`                      |
| Diff tint width, hunk row   | light | full width, 22 px lines                     | full width, 22 px lines                                     | match                       | explorer | `explorer-diff-light`                         |
| Diff layout                 | both  | all files in one scroll                     | one file per tab                                            | MISMATCH (Q-082)            | explorer | `explorer-diff-light`                         |
| Diff syntax colors          | dark  | `fn` purple, fn name blue                   | `fn` salmon, fn name lavender                               | MISMATCH (follow-up needed) | explorer | `explorer-diff-light`                         |
| Markdown highlight in diff  | both  | heading blue                                | plain text, no grammar                                      | MISMATCH (follow-up needed) | explorer | `explorer-diff-light`                         |
| File view                   | light | not captured                                | line numbers, colors, size line                             | UNVERIFIED (no reference)   | explorer | none                                          |
| Non-git, PNG, binary states | light | -                                           | "Not a git repository", image, "Binary preview unavailable" | match (GPUI only)           | explorer | none                                          |
| Live edit shows in Changes  | light | -                                           | within 3 s, on 6770                                         | match (not on live)         | explorer | none                                          |
| Dark Changes and diff       | dark  | surfaces `(26,25,23)`, header 36 px         | same                                                        | match                       | explorer | `explorer-files-dark`                         |

## Command center

| Element                    | Theme | Electron                        | GPUI                           | Result                    | Crate   | Pair                         |
| -------------------------- | ----- | ------------------------------- | ------------------------------ | ------------------------- | ------- | ---------------------------- |
| Panel, border, backdrop    | light | 640 x 560, `#e4e4e6`, pixel 127 | same                           | match                     | command | `command-center-light`       |
| Overlay top offset         | light | 48                              | 48                             | match                     | command | `command-center-light`       |
| Header height, input inset | light | 50, 17 px                       | 50, 17 px                      | match                     | command | `command-center-light`       |
| Row pitch, active row      | light | 36, `(251,251,251)` full width  | 36, `(251,251,251)` full width | match                     | command | `command-center-query-light` |
| Shortcut hint, icon rail   | light | right aligned, titles at x 325  | same                           | match                     | command | `command-center-query-light` |
| Section divider            | light | full width `#e1e1e1`            | full width                     | match                     | command | `command-center-query-light` |
| Row subtitles, file scope  | light | present                         | none                           | out of scope              | command | `command-center-light`       |
| Dark command center        | dark  | not captured                    | not paired                     | UNVERIFIED (no reference) | command | none                         |

## Settings

| Element                                    | Theme | Electron                                                 | GPUI                                  | Result                        | Crate    | Pair                        |
| ------------------------------------------ | ----- | -------------------------------------------------------- | ------------------------------------- | ----------------------------- | -------- | --------------------------- |
| Sidebar width, fill                        | light | 320, `#f4f4f6`                                           | 320, `#f4f4f6`                        | match                         | settings | `settings-hosts-light`      |
| Sidebar row pitch                          | light | 30 px                                                    | 30 px                                 | match                         | settings | `settings-appearance-light` |
| Page title spacing                         | light | 45 px to the section label                               | 46 px                                 | match                         | settings | `settings-appearance-light` |
| Theme dropdown, menu                       | light | 118 x 32, 14 px items, 16 px icons                       | same                                  | match                         | settings | `settings-theme-menu-light` |
| Theme list                                 | light | adds Zinc, Midnight, Claude, Ghostty, Pure black         | Light, Claude dark, System            | MISMATCH (by scope, ADR-0043) | settings | `settings-theme-menu-light` |
| Fonts size rows                            | light | 69 px, grey input                                        | 69 px, `#F4F4F6`, 1 px border         | match                         | settings | `settings-appearance-light` |
| Fonts rows missing                         | light | interface font, content size, code font, highlight theme | none                                  | MISMATCH (Q-083)              | settings | `settings-appearance-light` |
| Sidebar groups                             | light | App (12 pages) and Host groups                           | five pages                            | MISMATCH (Q-084)              | settings | `settings-hosts-light`      |
| Shortcut rows                              | light | 57 px, chord badge, `...` menu                           | 57 px, chord badge, `...` menu        | match                         | settings | `settings-shortcuts-light`  |
| Hosts add, remove, theme, rebind, relaunch | light | -                                                        | all persist                           | match (GPUI only)             | settings | none                        |
| Leave by Escape, Cmd+,                     | light | Escape leaves                                            | both leave, also with a field focused | match                         | app      | none                        |
| Providers, Projects pages                  | light | not captured                                             | not paired                            | UNVERIFIED (no reference)     | settings | none                        |
| Dark settings                              | dark  | not captured                                             | not paired                            | UNVERIFIED (no reference)     | settings | none                        |

## Terminal

| Element                | Theme | Electron                        | GPUI                   | Result                    | Crate    | Pair             |
| ---------------------- | ----- | ------------------------------- | ---------------------- | ------------------------- | -------- | ---------------- |
| Foreground, background | light | `#1b1b1b` on `#ffffff`          | `#161618` on `#ffffff` | match                     | terminal | `terminal-light` |
| Font, line height      | light | JetBrains Mono, 16 px           | same                   | match                     | terminal | `terminal-light` |
| Bold cyan and green    | light | brightened `#449dae`, `#55a874` | `#3ba2b4`, `#4fab71`   | match                     | terminal | `terminal-light` |
| Red, prompt arrow      | light | `#bb5d60`, `#53b17a`            | `#c2595a`, `#47b16d`   | match                     | terminal | `terminal-light` |
| Left padding           | light | 0 px                            | 0 px                   | match                     | terminal | `terminal-light` |
| 200000-line flood      | light | -                               | final state within 4 s | match (GPUI only)         | terminal | none             |
| Dark terminal          | dark  | not captured                    | not paired             | UNVERIFIED (no reference) | terminal | none             |

## Ranked mismatches

Highest user impact first.

1. Heading directly before a code block, `bachuc-timeline`: GPUI 25 to 26 px, Electron 37 px, an 11 to 12 px gap, in the agent text the owner reads most. Q-086 is open.
2. Header right buttons, `bachuc-app`: Electron has the VS Code split button and the `Commit` split button; GPUI has a `...` menu and a panel toggle. Follow-up needed (Commit menu needs the PR panel, which is out of scope).
3. Composer controls, `bachuc-composer`: @file mentions are still missing (Q-080). The lightning toggle, the diff-count pill, the track strip, and the plugin pills are built and wait for a UI check against Electron 6770.
4. Explorer, `bachuc-explorer`: no Files toolbar (36 px, so the first row sits 36 px higher), no `Commits` footer, no "added" mark (Q-081), one file per diff tab (Q-082). Follow-up needed for the toolbar and footer.
5. Diff syntax colors, `bachuc-explorer`: dark `fn` salmon `(224,135,127)` against Electron purple `(177,131,193)`; fn name lavender `(135,104,171)` against blue `(120,163,205)`. Markdown has no grammar. Follow-up needed.
6. Settings, `bachuc-settings`: four Fonts rows missing (Q-083), App and Host sidebar groups missing (Q-084), three themes against eight.
7. Sidebar, `bachuc-sidebar`: project letter colors (Q-077), stored project order (Q-085), pinned rows sort by tree order, not `pinnedAt`. A clean checkout shows no `+0 -0`.
8. Tab bar, `bachuc-workspace`: no tooltip, context menu, or maximize; the agent tab icon is a bot, not the Claude sparkle (follow-up needed); the divider grab band is 1 to 2 px off the drawn line (follow-up needed).
9. Find bar, `bachuc-timeline`: no 2 px focus ring on the field (follow-up needed).
10. Input log, gpui-base 0.7.3: every focused Input logs "has no accessibility node" (Q-087, defaulted to accept).
11. Layout drift, `bachuc-app`: first reply line ink is 2 to 4 px lower than Electron (224 against 222, 272 against 269, 369 against 365). Follow-up needed after Q-086.

## Out of scope surfaces

Per [CONTEXT.md](../../../watchtower/CONTEXT.md) Decisions. The Electron app has them; GPUI does not.

| Surface                     | Status       |
| --------------------------- | ------------ |
| Electron browser pane       | out of scope |
| Daemon management           | out of scope |
| Schedules                   | out of scope |
| Session import              | out of scope |
| Dictation                   | out of scope |
| Provider usage              | out of scope |
| PR panel                    | out of scope |
| Rewind and fork             | out of scope |
| Status-grouped sidebar list | out of scope |
| Sidebar display options     | out of scope |
| Drag-to-split               | out of scope |
| Open project flow           | out of scope |
| Skills picker               | out of scope |
| Send-behavior setting       | out of scope |

## UNVERIFIED

- Live down (6768): rename, pin, archive on a workspace; send, steer, queue, cancel; Stop button; answer or reject a plan or question card; CPU at a turn age of about 2 and 15 minutes (the reference daemon has no mock provider); tool rows for read, edit, write, search, fetch, and sub-agent.
- Needs pointer (owner rule, 2026-10-16): pick a step and expand a badge again; composer image pill; tab drag between groups; recovery from an offline host.
- Needs foreground (Q-069): a new screenshot of any surface or theme with no pair. No new capture was taken.
- No reference capture: dark theme for the agent pane, composer, tab bar, command center, settings, and terminal; file view; Providers and Projects settings pages.
- Idle CPU reads in the example apps passed (0.4 to 1.2 percent); the app-level read with a stream is open (Q-088).

## Manual checks

- VoiceOver pass (pending, needs a human): turn on VoiceOver with Cmd+F5, then walk the sidebar, one timeline message, and the composer. Compare the labels with the AX tree reads in the UI checks. Record the result here.

## Open owner questions

- Q-086 (TASK-149): allow a third fix round for the heading-to-code gap (25 px against 37 px), or accept it.
- Q-088 (TASK-155): start live and allow a foreground window for one CPU read at turn ages of 2 and 15 minutes, or run the read yourself.
