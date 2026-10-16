# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                         | Default                                                                  | Status    | Answer |
| ----- | -------- | ------ | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | --------- | ------ |
| Q-070 | TASK-145 | -      | `theme.ts` has no merged tint. Which color is `tint_merged`?                                                                     | `status_merged` at the alpha of the other status tints.                  | DEFAULTED | -      |
| Q-071 | TASK-145 | -      | Does `set_font_scale` take pixel sizes or factors of the default?                                                                | Factors of the defaults 14 px and 12 px; 1.0 is the default size.        | DEFAULTED | -      |
| Q-072 | TASK-146 | -      | Can the daemon send a newer agent state with an older `updatedAt`?                                                               | The client drops an upsert older than the stored one; none seen on live. | DEFAULTED | -      |
| Q-073 | TASK-146 | -      | A daemon permission name the client does not know makes `server_info()` return `None`, so queue support turns off. Keep this?    | Keep the closed enum; a new permission needs a client update.            | DEFAULTED | -      |
| Q-074 | TASK-147 | -      | `AgentSummary` has no `requires_attention`, so the sidebar never shows the Attention status. Add it to the client?               | Leave the Attention status out; a later TASK adds the field.             | DEFAULTED | -      |
| Q-075 | TASK-151 | -      | A detached HEAD gives `CheckoutStatus.branch` None, so Changes shows "Not a git repository". Add an `is_git` flag to the client? | Keep the non-git state for a detached HEAD; a later TASK adds the flag.  | DEFAULTED | -      |
