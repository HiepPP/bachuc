# TASK-074 Outcome

Status: DONE (auto: manual checks pending)

Autorun iteration 3: the lead re-ran the plugin tests (18 passed), typecheck, lint, and the plugin format check, all exit 0; the tests diff has 0 changed lines. The lead accepted the extra `shared/change-bar.ts` outside `Files:`: the node test runner cannot load the react-native row, and the helper stays inside the plugin. Class `code`, so no reviewer pass ran.

## Changed files

- `hiep-plugins/plugins/edit-diffs/client/edit-row.tsx`: one-line file row (chevron, mono path, change bar, +N −M), diff below with 6px gap; removed the outer `marginHorizontal: -13` and `marginBottom: 4` and the row padding, pencil icon, and verb label. A non-completed status (running, failed, canceled) still shows as a trailing muted/red word; a failed path is red. The thread switch, the locate RPC, and the `autoExpand` default source are unchanged.
- `hiep-plugins/plugins/edit-diffs/shared/change-bar.ts` (new): pure `changeBar(added, removed)` returning 7 squares, split by share, each changed side keeps at least 1 square.
- `hiep-plugins/plugins/edit-diffs/tests/change-bar.test.ts` (new): write -> 7 green, pure delete -> 7 red, mixed 4/3 split, tiny side keeps 1 square, no changes -> no bar.
- `hiep-plugins/plugins/edit-diffs/README.md`: one bullet on the row and change bar.

Deviation: `shared/change-bar.ts` is not in the spec's `Files:`. `edit-row.tsx` imports react-native, which the node test runner cannot load, so the testable split logic lives in `shared/` next to `diff.ts` (the plugin's existing pattern). Question for the owner: accept this extra file (Q-below).

## Impact (GitNexus)

- `EditDiffRow`: risk LOW, 1 direct caller (`contribute` in `index.client.tsx`), 0 processes. No HIGH/CRITICAL.

## Verify

- `cd hiep-plugins/plugins/edit-diffs && npm test` -> pass, 18 tests (13 before, +5 change bar), 0 fail.
- `npm run typecheck` -> exit 0. `npm run lint` -> exit 0 (0 warnings, 0 errors).
- `npm run cli -- plugin reload edit-diffs` -> exit 0; `plugin ls edit-diffs --json` shows `running`, enabled, no error (live daemon status was running on 6768).
- Manual on live (needs a screen): UNVERIFIED (autonomous run). Open an agent edit on live and check that the new file row shows with the diff open and that a click hides and shows it.
- `npm run format:files` / `format:check:files` from the repo root: both exit 2, "Expected at least one target file", because the root formatter ignores `hiep-plugins/`. Used the plugin's own `npm run format` (oxfmt) instead; `npx oxfmt --check client shared tests README.md` -> "All matched files use the correct format."

## Anti-goal

Result: PASS. Before changes `npm test` was 13 pass. At completion 18 pass. `git diff -U0 -- hiep-plugins/plugins/edit-diffs/tests` is empty: the existing test files are untouched, and the only test change is the new untracked `tests/change-bar.test.ts`. 0 edited assertions.

## Lessons

- The root `npm run format:files` skips `hiep-plugins/`; use the plugin's `npm run format` and `npx oxfmt --check`.
- A pure helper that a react-native file needs must live in `shared/` to be unit-tested with `tsx --test`.
- The `rtk` wrapper can hide lint output; redirect to a file and read the real exit code.
