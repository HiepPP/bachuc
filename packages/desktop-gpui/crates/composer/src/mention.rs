//! File mentions: the `@query` before the caret, the quoted path a pick or a drop puts in the
//! text, and the suggestion menu state. The rules port
//! `packages/app/src/utils/file-mention-autocomplete.ts`, and the request rules port
//! `useAgentAutocomplete` (`packages/app/src/hooks/use-agent-autocomplete.ts`).

use bachuc_client::DirectorySuggestion;
use std::path::Path;

/// The pause after the last key before a suggestion request, as `useAgentAutocomplete` waits.
pub const DEBOUNCE_MS: u64 = 180;
/// The suggestion count one request asks for.
pub const LIMIT: u32 = 50;
/// `agentAutocomplete.searchingWorkspace`.
pub const SEARCHING: &str = "Searching workspace...";
/// `agentAutocomplete.noFiles`.
pub const NO_FILES: &str = "No files or directories found";

/// An `@query` that ends at the caret. `start` is the byte offset of the `@`, `end` the caret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMention {
    pub start: usize,
    pub end: usize,
    pub query: String,
}

/// `findActiveFileMention`: the last `@` before the caret whose query has no whitespace or
/// quote. `cursor` is a byte offset; it is clamped to the text and to a char boundary.
pub fn find_active_file_mention(text: &str, cursor: usize) -> Option<FileMention> {
    let mut end = cursor.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    before.match_indices('@').rev().find_map(|(at, _)| {
        let query = &before[at + 1..];
        let invalid = query
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\'');
        (!invalid).then(|| FileMention {
            start: at,
            end,
            query: query.to_string(),
        })
    })
}

/// `formatQuotedFileMentionPath`: the path in double quotes, its own quotes escaped.
pub fn quoted_path(path: &str) -> String {
    format!("\"{}\"", path.replace('"', "\\\""))
}

/// `applyFileMentionReplacement`: the text with `@query` replaced by the quoted path, and the
/// caret after the closing quote.
pub fn apply_mention(text: &str, mention: &FileMention, path: &str) -> (String, usize) {
    let quoted = quoted_path(path);
    let caret = mention.start + quoted.len();
    let next = format!("{}{quoted}{}", &text[..mention.start], &text[mention.end..]);
    (next, caret)
}

/// The path a dropped or added file shows: relative to `cwd` when it lies inside it, else as
/// given.
pub fn mention_path(path: &Path, cwd: Option<&str>) -> String {
    cwd.filter(|cwd| !cwd.is_empty())
        .and_then(|cwd| path.strip_prefix(cwd).ok())
        .filter(|relative| !relative.as_os_str().is_empty())
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// The text with the quoted paths inserted at the caret, separated from the text around them
/// by a space, and the caret after the inserted text and its trailing space.
pub fn insert_mentions(text: &str, cursor: usize, paths: &[String]) -> (String, usize) {
    let mut at = cursor.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    let (before, after) = text.split_at(at);
    let mut inserted = String::new();
    if before
        .chars()
        .next_back()
        .is_some_and(|c| !c.is_whitespace())
    {
        inserted.push(' ');
    }
    let quoted: Vec<String> = paths.iter().map(|path| quoted_path(path)).collect();
    inserted.push_str(&quoted.join(" "));
    if !after.starts_with(char::is_whitespace) {
        inserted.push(' ');
    }
    (format!("{before}{inserted}{after}"), at + inserted.len())
}

/// A suggestion request the composer sends after the debounce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuggestionRequest {
    pub cwd: String,
    pub query: String,
    pub generation: u64,
}

/// The `@` menu: the last answer, the highlighted row, and which mention it belongs to.
#[derive(Clone, Debug, Default)]
pub struct MentionMenu {
    /// The start and query of the mention the last request asked for.
    asked: Option<(usize, String)>,
    pub items: Vec<DirectorySuggestion>,
    pub loading: bool,
    pub error: Option<String>,
    /// The highlighted row, best match first.
    pub active: usize,
    /// Escape closed the menu for the mention that starts here.
    dismissed: Option<usize>,
    generation: u64,
}

impl MentionMenu {
    /// After a text change: the request to send for the active mention, or `None` when nothing
    /// new needs asking. Without a folder there is no menu and no request.
    pub fn update(
        &mut self,
        cwd: Option<&str>,
        mention: Option<&FileMention>,
    ) -> Option<SuggestionRequest> {
        let (Some(cwd), Some(mention)) = (cwd.filter(|cwd| !cwd.is_empty()), mention) else {
            let dismissed = mention.and(self.dismissed);
            *self = MentionMenu {
                generation: self.generation + 1,
                dismissed,
                ..MentionMenu::default()
            };
            return None;
        };
        match self.dismissed {
            Some(start) if start == mention.start => return None,
            _ => self.dismissed = None,
        }
        let key = (mention.start, mention.query.clone());
        if self.asked.as_ref() == Some(&key) {
            return None;
        }
        self.asked = Some(key);
        self.loading = true;
        self.error = None;
        self.active = 0;
        self.generation += 1;
        Some(SuggestionRequest {
            cwd: cwd.to_string(),
            query: mention.query.clone(),
            generation: self.generation,
        })
    }

    /// Takes an answer; one for an older request is dropped. Returns whether it was taken.
    pub fn accept(
        &mut self,
        generation: u64,
        answer: Result<Vec<DirectorySuggestion>, String>,
    ) -> bool {
        if generation != self.generation {
            return false;
        }
        self.loading = false;
        match answer {
            Ok(items) => {
                self.items = items;
                self.error = None;
            }
            Err(error) => {
                self.items.clear();
                self.error = Some(error);
            }
        }
        self.active = 0;
        true
    }

    /// Whether the menu shows for the mention at the caret.
    pub fn open(&self, cwd: Option<&str>, mention: Option<&FileMention>) -> bool {
        let Some(mention) = mention else {
            return false;
        };
        cwd.is_some_and(|cwd| !cwd.is_empty())
            && self.dismissed != Some(mention.start)
            && self
                .asked
                .as_ref()
                .is_some_and(|(start, _)| *start == mention.start)
    }

    pub fn dismiss(&mut self, mention: &FileMention) {
        self.dismissed = Some(mention.start);
    }

    /// The text the menu shows in place of rows; `None` when it has rows.
    pub fn status(&self) -> Option<String> {
        if !self.items.is_empty() {
            return None;
        }
        if self.loading {
            return Some(SEARCHING.to_string());
        }
        Some(self.error.clone().unwrap_or_else(|| NO_FILES.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mention(start: usize, end: usize, query: &str) -> FileMention {
        FileMention {
            start,
            end,
            query: query.to_string(),
        }
    }

    fn entry(path: &str) -> DirectorySuggestion {
        DirectorySuggestion {
            path: path.to_string(),
            is_directory: false,
        }
    }

    #[test]
    fn detects_mentions_at_the_start_of_input() {
        let text = "@src/components";
        assert_eq!(
            find_active_file_mention(text, text.len()),
            Some(mention(0, text.len(), "src/components"))
        );
    }

    #[test]
    fn detects_mentions_in_the_middle_of_input_using_cursor_position() {
        let text = "read \"@src/com\" before merging";
        let cursor = text.find('"').unwrap() + 9;
        assert_eq!(
            find_active_file_mention(text, cursor),
            Some(mention(text.find('@').unwrap(), cursor, "src/com"))
        );
    }

    #[test]
    fn returns_none_when_cursor_is_outside_the_mention_token() {
        let text = "please review @src/components now";
        assert_eq!(find_active_file_mention(text, text.len()), None);
    }

    #[test]
    fn returns_none_when_at_at_start_is_followed_by_a_delimiter() {
        assert_eq!(find_active_file_mention("@ ", 2), None);
        assert_eq!(find_active_file_mention("@'x", 3), None);
    }

    #[test]
    fn a_bare_at_after_a_space_is_an_empty_mention_and_a_space_ends_it() {
        assert_eq!(
            find_active_file_mention("see @", 5),
            Some(mention(4, 5, ""))
        );
        assert_eq!(
            find_active_file_mention("see @src/ma", 11),
            Some(mention(4, 11, "src/ma"))
        );
        assert_eq!(find_active_file_mention("see @src ", 9), None);
    }

    #[test]
    fn the_last_valid_at_before_the_caret_wins() {
        // The text between the earlier `@` and the caret holds the later `@` and no space.
        assert_eq!(find_active_file_mention("a@b@c d", 7), None);
        assert_eq!(
            find_active_file_mention("a@b@c", 5),
            Some(mention(3, 5, "c"))
        );
    }

    #[test]
    fn the_cursor_clamps_to_the_text_and_to_a_char_boundary() {
        assert_eq!(find_active_file_mention("@é", 99), Some(mention(0, 3, "é")));
        assert_eq!(find_active_file_mention("@é", 2), Some(mention(0, 1, "")));
    }

    #[test]
    fn quotes_workspace_relative_paths_using_file_mention_escaping() {
        assert_eq!(
            quoted_path("src/changed \"file\".ts"),
            "\"src/changed \\\"file\\\".ts\""
        );
    }

    #[test]
    fn replaces_only_the_active_mention_segment_with_a_quoted_relative_path() {
        let text = "open @src/com next";
        let (next, caret) =
            apply_mention(text, &mention(5, 13, "src/com"), "src/components/chat.tsx");
        assert_eq!(next, "open \"src/components/chat.tsx\" next");
        assert_eq!(&next[..caret], "open \"src/components/chat.tsx\"");
    }

    #[test]
    fn escapes_double_quotes_in_the_replacement_path() {
        let (next, _) = apply_mention("@foo", &mention(0, 4, "foo"), "src/\"quoted\".ts");
        assert_eq!(next, "\"src/\\\"quoted\\\".ts\"");
    }

    #[test]
    fn a_dropped_path_inside_the_folder_is_relative_and_outside_absolute() {
        let cwd = Some("/Users/me/project");
        assert_eq!(
            mention_path(Path::new("/Users/me/project/src/x.rs"), cwd),
            "src/x.rs"
        );
        assert_eq!(
            mention_path(Path::new("/Users/me/project/"), cwd),
            "/Users/me/project/"
        );
        assert_eq!(
            mention_path(Path::new("/Users/me/projects/x.rs"), cwd),
            "/Users/me/projects/x.rs"
        );
        assert_eq!(mention_path(Path::new("/tmp/x.rs"), None), "/tmp/x.rs");
        assert_eq!(mention_path(Path::new("src/x.rs"), cwd), "src/x.rs");
    }

    #[test]
    fn inserted_mentions_get_spaces_around_them() {
        let paths = vec!["src/x.rs".to_string()];
        assert_eq!(insert_mentions("", 0, &paths), ("\"src/x.rs\" ".into(), 11));
        let (text, caret) = insert_mentions("see", 3, &paths);
        assert_eq!(text, "see \"src/x.rs\" ");
        assert_eq!(caret, text.len());
        let (text, caret) = insert_mentions("see  now", 4, &paths);
        assert_eq!(text, "see \"src/x.rs\" now");
        assert_eq!(&text[caret..], " now");
        let two = vec!["a".to_string(), "b".to_string()];
        assert_eq!(insert_mentions("x", 0, &two).0, "\"a\" \"b\" x");
    }

    #[test]
    fn a_mention_without_a_folder_sends_no_request_and_shows_no_menu() {
        let mut menu = MentionMenu::default();
        let at = mention(0, 1, "");
        assert_eq!(menu.update(None, Some(&at)), None);
        assert!(!menu.open(None, Some(&at)));
        assert_eq!(menu.update(Some(""), Some(&at)), None);
        assert!(!menu.open(Some(""), Some(&at)));
    }

    #[test]
    fn a_mention_asks_once_per_query_and_takes_only_the_latest_answer() {
        let mut menu = MentionMenu::default();
        let cwd = Some("/p");
        let first = menu.update(cwd, Some(&mention(4, 6, "s"))).unwrap();
        assert_eq!((first.cwd.as_str(), first.query.as_str()), ("/p", "s"));
        assert!(menu.open(cwd, Some(&mention(4, 6, "s"))));
        assert_eq!(menu.status().as_deref(), Some(SEARCHING));
        // The same mention again (a change elsewhere) asks nothing.
        assert_eq!(menu.update(cwd, Some(&mention(4, 6, "s"))), None);
        let second = menu.update(cwd, Some(&mention(4, 7, "sr"))).unwrap();
        assert!(!menu.accept(first.generation, Ok(vec![entry("old")])));
        assert!(menu.accept(second.generation, Ok(vec![entry("src/main.rs")])));
        assert_eq!(menu.items, vec![entry("src/main.rs")]);
        assert_eq!(menu.status(), None);
    }

    #[test]
    fn leaving_the_mention_closes_the_menu_and_drops_the_pending_answer() {
        let mut menu = MentionMenu::default();
        let cwd = Some("/p");
        let request = menu.update(cwd, Some(&mention(0, 2, "s"))).unwrap();
        assert_eq!(menu.update(cwd, None), None);
        assert!(!menu.open(cwd, None));
        assert!(!menu.accept(request.generation, Ok(vec![entry("x")])));
        assert!(menu.items.is_empty());
    }

    #[test]
    fn escape_closes_the_menu_until_another_mention_starts() {
        let mut menu = MentionMenu::default();
        let cwd = Some("/p");
        let at = mention(0, 2, "s");
        menu.update(cwd, Some(&at));
        menu.dismiss(&at);
        assert!(!menu.open(cwd, Some(&at)));
        menu.update(cwd, Some(&mention(0, 3, "sr")));
        assert!(!menu.open(cwd, Some(&mention(0, 3, "sr"))));
        let later = mention(5, 6, "");
        menu.update(cwd, Some(&later));
        assert!(menu.open(cwd, Some(&later)));
    }

    #[test]
    fn an_empty_or_failed_answer_shows_its_status() {
        let mut menu = MentionMenu::default();
        let request = menu.update(Some("/p"), Some(&mention(0, 1, ""))).unwrap();
        menu.accept(request.generation, Ok(Vec::new()));
        assert_eq!(menu.status().as_deref(), Some(NO_FILES));
        let request = menu.update(Some("/p"), Some(&mention(0, 2, "z"))).unwrap();
        menu.accept(request.generation, Err("boom".into()));
        assert_eq!(menu.status().as_deref(), Some("boom"));
    }
}
