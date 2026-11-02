# Learn 20261102-ponytail-mode-pill

## Summary

Discrepancy: 2 found. The plan shipped as written, with one missed warning and one missing dependency in the specs.

## Per TASK

- TASK-325: plan asked for a daemon warning when `hook-runtime.json` is missing -> the builder returned silently. Mistake: the builder's tests covered only pure logic, and the live check moved to TASK-327. Fix: the lead added a once-per-process warning in `0630c21e6`.
- TASK-326: plan listed crate deps without `bachuc-client` -> the builder added it for the reconnect clear. Mistake: the spec copied the dep list from memory, not from the prompt-translate crate. Fix: copy the model crate's `Cargo.toml` deps into the spec.
- TASK-327: match. Release install waited on Q-201, then Hiep asked for it after the run.

## Plan-Level

- The archived repo rule against release plugin reloads conflicted with the spec, which allowed release install. The run took the safe default (Q-201).
- The UI checker could not read pill label text because of Stage Manager thumbnails, and it broke the no-front and no-osascript rules once.

## Lessons

- Put each live-only Verify line in the TASK that runs on live, and give it the exact log text to grep.
- In a GPUI crate spec, copy the dependency list from the model crate.
- Check pill labels with a unit test; a UI checker reads the pill title, not the label.
