# TASK-286 Composer prompt history with Up and Down

Group: A (writes the `bachuc-composer` crate only)
Class: code

## Brief

Goal: In the GPUI composer, Up and Down recall the prompts Hiep sent earlier in the same repo, as in the Claude Code CLI.

Change: no prompt history -> each sent prompt is saved per repo, and Up/Down at the first/last line walk through it.

Boundaries:

- Follows [ADR-0060](watchtower/decisions/ADR-0060-gpui-local-prompt-history-per-repo-over-daemon.md): history lives in `prompt-history.json` in the GPUI support folder (`data_dir()` in [packages/desktop-gpui/crates/command/src/shortcuts.rs](packages/desktop-gpui/crates/command/src/shortcuts.rs)), keyed by the project root.
- The key is the `root` of the `ProjectNode` that has a workspace whose `cwd` equals the composer cwd, or whose `root` equals it. Compare paths without a trailing `/`. With no match, the key is the cwd itself. So all worktrees of one repo share one list.
- The composer cwd is `draft.options.cwd` for a draft and the agent cwd that `mention_cwd` in [composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) reads for an agent. The projects come from `Client::projects()`. With no cwd, history is off.
- Up recalls only when the cursor is on the first line. Down recalls only when the cursor is on the last line. Other times the textarea moves the cursor as before.
- The first Up stashes the current text. Down past the newest entry puts the stash back.
- The slash menu and the mention menu keep Up and Down while they are open. History runs only when both are closed.
- Record the typed text at the start of `submit`, after its `processing` and `locked` guard and before the client command, plugin command, and interceptor paths. So slash commands, drafts, and queued sends are all kept, as typed, as in the CLI. Skip empty or whitespace-only text. Skip a prompt equal to the newest entry of that key.
- Planner call: while IME composition is active, Up and Down go to the textarea, because `InputState::cursor()` gives the marked range end then.
- Keep at most 200 entries per key; drop the oldest.
- Read the file once per composer build or from a shared cache, never per key press. Write it off the UI thread, through a temp file and a rename, as `LayoutPrefs::save_to` in [packages/desktop-gpui/crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) does.
- Planner call: after a recall, put the cursor at the end of the text. A recalled multi-line prompt then needs Up presses inside the text before the next older entry, as in the CLI.
- Planner call: a submit or a switch to another agent resets the walk. Edits to a recalled entry do not change the saved history.
- Non-goal: Ctrl+R search, Electron or mobile history, and a daemon RPC.
- Trap: the keystroke intercept in `Composer::new` in [composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) returns early on any modifier. Keep that, so Shift+Up still selects text.
- Trap: `InputState::cursor()` in [packages/desktop-gpui/vendor/gpui-base/src/input/base/state.rs](packages/desktop-gpui/vendor/gpui-base/src/input/base/state.rs) gives a UTF-8 byte offset. Slice the text only at that offset, never at a char count.

How:

- Add a pure module `prompt_history.rs` with `PromptHistory` (`parse`, `to_json`, `push`, `entries`, load and save to a folder), `MAX_ENTRIES`, `Recall` (`up`, `down`, `reset`), `history_key(cwd: &str, projects: &[ProjectNode]) -> String`, `on_first_line(text: &str, cursor: usize) -> bool`, and `on_last_line(text: &str, cursor: usize) -> bool`.
- In `composer.rs`, add a history step to the keystroke intercept after `mention_key` and `slash_key`, and call `PromptHistory::push` at the start of `submit`.

Files:

- [packages/desktop-gpui/crates/composer/src/prompt_history.rs](packages/desktop-gpui/crates/composer/src/prompt_history.rs) (new: history model, walk state, key lookup, line checks, file load and save, unit tests)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (Up/Down handling in the keystroke intercept; record on submit)
- [packages/desktop-gpui/crates/composer/src/lib.rs](packages/desktop-gpui/crates/composer/src/lib.rs) (add `mod prompt_history;`)

Expected result:

- WHEN Hiep sends a prompt THE composer SHALL add it as the newest entry of the repo key and save the file.
- WHILE the cursor is on the first line and no menu is open, WHEN Hiep presses Up THE composer SHALL replace the text with the next older entry.
- WHILE a recall is active and the cursor is on the last line, WHEN Hiep presses Down THE composer SHALL show the next newer entry, or the stashed text after the newest.
- WHILE the slash or mention menu is open, WHEN Hiep presses Up or Down THE menu SHALL move its selection and the text SHALL not change.
- WHEN a composer in another worktree of the same repo opens THE composer SHALL recall the same entries.
- IF the key has no entries, or the file is missing or bad, THEN Up SHALL leave the text unchanged.
- IF the sent text is empty or equals the newest entry THEN THE history SHALL not change.

Red tests:

File: [packages/desktop-gpui/crates/composer/src/prompt_history.rs](packages/desktop-gpui/crates/composer/src/prompt_history.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_client::{ProjectNode, WorkspaceNode};

    fn entries(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn push_skips_empty_and_a_repeat_of_the_newest() {
        let mut history = PromptHistory::default();
        history.push("/repo", "first");
        history.push("/repo", "   ");
        history.push("/repo", "first");
        history.push("/repo", "second");
        assert_eq!(history.entries("/repo"), entries(&["first", "second"]).as_slice());
        assert!(history.entries("/other").is_empty());
    }

    #[test]
    fn push_keeps_the_newest_two_hundred() {
        let mut history = PromptHistory::default();
        for n in 0..205 {
            history.push("/repo", &format!("p{n}"));
        }
        let kept = history.entries("/repo");
        assert_eq!(kept.len(), MAX_ENTRIES);
        assert_eq!(kept.first().map(String::as_str), Some("p5"));
        assert_eq!(kept.last().map(String::as_str), Some("p204"));
    }

    #[test]
    fn a_bad_file_gives_an_empty_history_and_json_round_trips() {
        assert!(PromptHistory::parse("not json").entries("/repo").is_empty());
        let mut history = PromptHistory::default();
        history.push("/repo", "line one\nline two");
        let back = PromptHistory::parse(&history.to_json());
        assert_eq!(back.entries("/repo"), entries(&["line one\nline two"]).as_slice());
    }

    #[test]
    fn up_walks_older_and_down_returns_the_stash() {
        let list = entries(&["a", "b", "c"]);
        let mut recall = Recall::default();
        assert_eq!(recall.up(&list, "draft").as_deref(), Some("c"));
        assert_eq!(recall.up(&list, "c").as_deref(), Some("b"));
        assert_eq!(recall.up(&list, "b").as_deref(), Some("a"));
        assert_eq!(recall.up(&list, "a"), None);
        assert_eq!(recall.down(&list).as_deref(), Some("b"));
        assert_eq!(recall.down(&list).as_deref(), Some("c"));
        assert_eq!(recall.down(&list).as_deref(), Some("draft"));
        assert_eq!(recall.down(&list), None);
    }

    #[test]
    fn up_with_no_entries_does_nothing() {
        let mut recall = Recall::default();
        assert_eq!(recall.up(&[], "draft"), None);
        assert_eq!(recall.down(&[]), None);
    }

    #[test]
    fn reset_starts_a_new_walk() {
        let list = entries(&["a", "b"]);
        let mut recall = Recall::default();
        recall.up(&list, "draft");
        recall.reset();
        assert_eq!(recall.up(&list, "new draft").as_deref(), Some("b"));
        assert_eq!(recall.down(&list).as_deref(), Some("new draft"));
    }

    #[test]
    fn line_checks_use_the_cursor_offset() {
        let text = "one\ntwo\nthree";
        assert!(on_first_line(text, 2));
        assert!(!on_first_line(text, 5));
        assert!(on_last_line(text, text.len()));
        assert!(!on_last_line(text, 5));
        assert!(on_first_line("", 0) && on_last_line("", 0));
        assert!(on_first_line("é\nx", 2));
        assert!(!on_last_line("é\nx", 2));
    }

    #[test]
    fn a_worktree_cwd_keys_to_its_project_root() {
        let projects = vec![ProjectNode {
            id: "p1".into(),
            name: "bachuc".into(),
            root: "/Users/hiep/Projects/bachuc".into(),
            workspaces: vec![WorkspaceNode {
                id: "w1".into(),
                cwd: "/Users/hiep/.bachuc/worktrees/bachuc/feat-x".into(),
                ..Default::default()
            }],
        }];
        assert_eq!(
            history_key("/Users/hiep/.bachuc/worktrees/bachuc/feat-x", &projects),
            "/Users/hiep/Projects/bachuc"
        );
        assert_eq!(history_key("/Users/hiep/Projects/bachuc", &projects), "/Users/hiep/Projects/bachuc");
        assert_eq!(history_key("/Users/hiep/Projects/bachuc/", &projects), "/Users/hiep/Projects/bachuc");
        assert_eq!(history_key("/tmp/loose", &projects), "/tmp/loose");
    }
}
```

Anti-goal: existing `bachuc-composer` tests must keep passing, with 0 new failures against the count on `main`; tripwire; read from `./cargo.sh test -p bachuc-composer` before the change, after the key handling change, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer prompt_history` -> every red test above passes.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer` -> 0 failures, the same as the baseline read.
- `rg -n "std::fs::read" packages/desktop-gpui/crates/composer/src/composer.rs` -> no read inside the keystroke intercept closure.
- Screen check on live (needs the GPUI app and a screen, per Screen checks in [docs/gpui.md](docs/gpui.md)): send two prompts in one workspace, open a worktree workspace of the same repo, press Up twice -> the composer shows the second, then the first prompt; Down twice -> the empty draft comes back.
- Screen check on live: type `/` to open the slash menu and press Down -> the menu selection moves and the text does not change.
- Screen check on live: type three lines, put the cursor on line two, press Up -> the cursor moves to line one and the text does not change.
- Screen check on live: delete `prompt-history.json`, open a composer, press Up -> nothing changes and no error shows.
