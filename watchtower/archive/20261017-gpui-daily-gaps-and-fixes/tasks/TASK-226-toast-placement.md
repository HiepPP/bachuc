# TASK-226 Toasts at the top center with Electron timing

Group: S (shared file [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs), whose timer at line 1619 uses `TOAST_DURATION`; runs after TASK-231)
Class: code

## Brief

Goal: Toasts show where and as long as Electron's do.

Change: bottom right, 3.2 s for every toast -> top center, 2.2 s for success, 3.2 s for errors, max width 480. Q-130.

Boundaries:

- Electron: [packages/app/src/components/toast-host.tsx](packages/app/src/components/toast-host.tsx) lines 54-55, 108-113, 276-278.
- GPUI today: `toast.rs` line 12 (`TOAST_DURATION`), line 14 (`MAX_TOASTS`), line 15 (max width 480, already right), lines 106-108 (bottom right). `shell.rs` imports `TOAST_DURATION` (line 29) and uses it for the timer (line 1619). Planner call: each toast carries its own duration; `shell.rs` reads it.
- Toast count: follow Electron's toast host for how many toasts show at once.
- `Shell::show_toast(ok, message, cx)` keeps its signature. Put the duration rule in a pure function with a test.

Files:

- [packages/desktop-gpui/crates/app/src/toast.rs](packages/desktop-gpui/crates/app/src/toast.rs)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (timer duration and the `show_toast` doc line)

Expected result:

- A success toast shows at the top center and leaves after 2.2 s; an error toast after 3.2 s.

Anti-goal: a toast adds no work while none shows; tripwire; no timer or repaint runs with an empty toast list (read the code before and after).

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app toast` -> passes, with the duration rule.
- UI check on live: a commit in the scratch repo shows the toast at the top center.
