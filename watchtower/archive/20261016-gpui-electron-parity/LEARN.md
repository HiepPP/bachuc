# Learn 20261016-gpui-electron-parity

## Summary

Discrepancy: 6 found. All 12 TASKs shipped code on `feat/bachuc-gpui`; screen, live, and CPU checks fell short because of a crash, a down live daemon, and owner rules on the cursor and the foreground.

## Per TASK

- TASK-145: match. One example gap: the tokens example toolbar does not repaint on a theme switch.
- TASK-146: plan said an add-only typed `server_info` -> shipped a closed permission enum that dropped `server_info` on an unknown name. Mistake: the spec did not pin the compat rule of `docs/protocol-compatibility.md`. Fix: shipped in `53339d4cc` (Q-073); pin compat rules in Verify next time.
- TASK-147: plan said Electron sidebar parity -> shipped without letter colors, stored order, and the Attention status. Mistake: the client lacks `projectKey`, `diffStat`, and `requires_attention`. Fix: follow-up plan (Q-074, Q-077, Q-078).
- TASK-148: match.
- TASK-149: plan said parity and no regressions -> the first build crashed on a tool badge (display toggled from group hover) and a heading-to-fence gap needed a third fix round. Mistake: hover-driven `display` and gpui margin collapse were unknown. Fix: shipped in `abf2569f1` and `bdaafa912`; the 37 px gap has a unit test but no screen check.
- TASK-150: match, with known gaps Q-079 and Q-080.
- TASK-151: plan said lazy explorer parity -> the first build sent requests before connect and showed "not connected". Mistake: the client does not retry and the spec did not say to wait for `Connected`. Fix: shipped in `705b30487`; follow-up for the Files toolbar, Commits footer, and multi-file diff (Q-082).
- TASK-152: match after two fix rounds; the gpui-kit `Input::h` shadows `Styled::h`.
- TASK-153: match after two fix rounds; GPUI's default 1.618 line height differs from the browser's 1.2.
- TASK-154: match after one fix round (bold text in bright colors).
- TASK-155: plan said CPU at most 10% at about 2 and 15 minutes -> never measured. Mistake: the read needs live and a foreground window, and the owner stopped a foreground run that blocked other apps. Fix: the owner measures CPU by hand (Q-088 OPEN).
- TASK-156: plan said both themes for every surface -> most dark rows are UNVERIFIED. Mistake: dark pairs need the foreground. Fix: take dark pairs when the owner is away.

## Plan-Level

- Scheduling: 6 builders plus a UI checker on a 24 GB Mac ran out of RAM and swap and panicked. The run restarted with 2 builders and `CARGO_BUILD_JOBS=4`.
- Plan misdescribes reality: CONTEXT told checkers to keep GPUI in front with an `open` loop. The owner works on the same Mac, so screen checks must not take the cursor or the foreground.
- Dependency gap: several parity items need new client fields; the plan did not list them, so they became DEFAULTED questions and a follow-up plan.
- Naming: "App integration and Electron window chrome" read as Electron integration; renamed to "App shell integration with the Electron look".

## Lessons

- Run at most 2 builders on this machine, and keep run files out of `/tmp`.
- Write screen checks that need no cursor and no foreground, or schedule them for a time the owner is away.
- In GPUI, never toggle `display` from hover, give every `track_focus` element a role, and add spacing with padding or offsets where margins collapse.
- Pin protocol compat rules and "wait for Connected" in the Verify lines of client-facing TASKs.
