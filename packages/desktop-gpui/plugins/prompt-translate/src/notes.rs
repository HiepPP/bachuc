//! The note under a prompt and its cache (`client/notes.ts`). Vietnamese text gets its English
//! translation; an English rewrite gets its original draft. The cache holds no `App`: a caller
//! asks, starts the lookup when told to, and settles it with the answer.

use crate::vietnamese::has_vietnamese;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Label {
    En,
    ViOriginal,
    EnOriginal,
}

impl Label {
    pub(crate) fn text(self) -> &'static str {
        match self {
            Label::En => "EN",
            Label::ViOriginal => "VI gốc",
            Label::EnOriginal => "EN gốc",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Note {
    pub(crate) label: Label,
    pub(crate) text: String,
}

/// The lookup a text needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Request {
    Translate { text: String, cache_only: bool },
    Original { text: String },
}

pub(crate) fn request_for(text: &str, cache_only: bool) -> Request {
    if has_vietnamese(text) {
        Request::Translate {
            text: text.into(),
            cache_only,
        }
    } else {
        Request::Original { text: text.into() }
    }
}

/// One entry per distinct text and cache mode.
pub(crate) fn cache_key(text: &str, cache_only: bool) -> String {
    format!("{}\0{text}", if cache_only { "cached" } else { "live" })
}

/// The note an answer makes, or `None` for a `null` (or empty) answer.
pub(crate) fn note_from_answer(text: &str, answer: Option<String>) -> Option<Note> {
    let value = answer.filter(|value| !value.is_empty())?;
    let label = if has_vietnamese(text) {
        Label::En
    } else if has_vietnamese(&value) {
        Label::ViOriginal
    } else {
        Label::EnOriginal
    };
    Some(Note { label, text: value })
}

/// A prompt sent before `active_since` never starts a new paid call. A missing or unparsable
/// timestamp counts as before it.
pub(crate) fn is_cache_only(timestamp: Option<&str>, active_since: i64) -> bool {
    timestamp
        .and_then(|iso| chrono::DateTime::parse_from_rfc3339(iso).ok())
        .is_none_or(|time| time.timestamp_millis() < active_since)
}

enum Slot<W> {
    Pending { text: String, waiters: Vec<W> },
    Done(Option<Note>),
}

/// What an ask gets.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Ask {
    Ready(Option<Note>),
    /// Another view started the lookup; the waiter hears when it settles.
    Wait,
    /// The caller sends `request`, then calls `finish(key, ..)`.
    Start {
        key: String,
        request: Request,
    },
}

/// How a lookup ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Settled(Option<Note>),
    Failed,
}

/// One lookup per text and cache mode, shared by every row (`createNoteCache`). `W` names a
/// view that waits for a lookup another view started.
pub(crate) struct Cache<W> {
    slots: BTreeMap<String, Slot<W>>,
}

impl<W> Default for Cache<W> {
    fn default() -> Self {
        Self {
            slots: BTreeMap::new(),
        }
    }
}

impl<W> Cache<W> {
    pub(crate) fn ask(&mut self, text: &str, cache_only: bool, waiter: W) -> Ask {
        let key = cache_key(text, cache_only);
        match self.slots.get_mut(&key) {
            Some(Slot::Done(note)) => Ask::Ready(note.clone()),
            Some(Slot::Pending { waiters, .. }) => {
                waiters.push(waiter);
                Ask::Wait
            }
            None => {
                self.slots.insert(
                    key.clone(),
                    Slot::Pending {
                        text: text.into(),
                        waiters: vec![waiter],
                    },
                );
                Ask::Start {
                    key,
                    request: request_for(text, cache_only),
                }
            }
        }
    }

    /// Settles a lookup and returns the views that waited. A failed lookup leaves the cache, so
    /// a later ask tries again.
    pub(crate) fn finish(
        &mut self,
        key: &str,
        answer: anyhow::Result<Option<String>>,
    ) -> (Vec<W>, Outcome) {
        let Some(Slot::Pending { text, waiters }) = self.slots.remove(key) else {
            return (Vec::new(), Outcome::Failed);
        };
        match answer {
            Ok(answer) => {
                let note = note_from_answer(&text, answer);
                self.slots.insert(key.into(), Slot::Done(note.clone()));
                (waiters, Outcome::Settled(note))
            }
            Err(error) => {
                log::debug!("prompt-translate lookup failed: {error:#}");
                (waiters, Outcome::Failed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Ask, Cache, Label, Note, Outcome, Request, cache_key, is_cache_only, note_from_answer,
    };

    /// The lookups of `notes.test.ts`, answered synchronously and counted.
    struct Backend {
        cache: Cache<u8>,
        calls: Vec<String>,
        fail_first: bool,
    }

    impl Backend {
        fn new() -> Self {
            Self {
                cache: Cache::default(),
                calls: Vec::new(),
                fail_first: false,
            }
        }

        fn lookup(&mut self, text: &str, cache_only: bool) -> Option<Note> {
            match self.cache.ask(text, cache_only, 0) {
                Ask::Ready(note) => note,
                Ask::Wait => unreachable!("answers are synchronous"),
                Ask::Start { key, request } => {
                    let answer = match &request {
                        Request::Translate { text, .. } => {
                            self.calls.push(format!("translate:{text}"));
                            if self.fail_first {
                                self.fail_first = false;
                                Err(anyhow::anyhow!("offline"))
                            } else {
                                Ok(Some("Explain".to_string()))
                            }
                        }
                        Request::Original { text } => {
                            self.calls.push(format!("original:{text}"));
                            Ok(match text.as_str() {
                                "Fix the bug." => Some("fix teh bug".to_string()),
                                "Explain the bug" => Some("Giải thích lỗi".to_string()),
                                _ => None,
                            })
                        }
                    };
                    match self.cache.finish(&key, answer).1 {
                        Outcome::Settled(note) => note,
                        Outcome::Failed => None,
                    }
                }
            }
        }
    }

    fn note(label: Label, text: &str) -> Option<Note> {
        Some(Note {
            label,
            text: text.into(),
        })
    }

    #[test]
    fn each_distinct_text_is_looked_up_once_across_renders() {
        let mut backend = Backend::new();
        for _ in 0..5 {
            assert_eq!(
                backend.lookup("Giải thích", false),
                note(Label::En, "Explain")
            );
            assert_eq!(
                backend.lookup("Explain the bug", false),
                note(Label::ViOriginal, "Giải thích lỗi")
            );
            assert_eq!(
                backend.lookup("Fix the bug.", false),
                note(Label::EnOriginal, "fix teh bug")
            );
            assert_eq!(backend.lookup("Plain English", false), None);
        }
        assert_eq!(
            backend.calls,
            [
                "translate:Giải thích",
                "original:Explain the bug",
                "original:Fix the bug.",
                "original:Plain English",
            ]
        );
    }

    #[test]
    fn a_failed_lookup_is_retried_on_the_next_render() {
        let mut backend = Backend::new();
        backend.fail_first = true;
        assert_eq!(backend.lookup("Giải thích", false), None);
        assert_eq!(
            backend.lookup("Giải thích", false),
            note(Label::En, "Explain")
        );
        assert_eq!(backend.calls.len(), 2);
    }

    #[test]
    fn failed_lookup_retries() {
        let mut cache: Cache<u8> = Cache::default();
        let Ask::Start { key, .. } = cache.ask("Giải thích", false, 1) else {
            panic!("the first ask starts the lookup");
        };
        // A second view waits on the first lookup instead of starting its own.
        assert_eq!(cache.ask("Giải thích", false, 2), Ask::Wait);
        let (waiters, outcome) = cache.finish(&key, Err(anyhow::anyhow!("offline")));
        assert_eq!(waiters, [1, 2]);
        assert_eq!(outcome, Outcome::Failed);
        // The entry is gone, so the next ask starts a new lookup.
        assert!(matches!(
            cache.ask("Giải thích", false, 1),
            Ask::Start { .. }
        ));
    }

    #[test]
    fn note_cache_one_lookup() {
        // The anti-goal: 100 renders of one note make one RPC.
        let mut cache: Cache<u32> = Cache::default();
        let mut starts = 0;
        for render in 0..100 {
            match cache.ask("Giải thích", false, render) {
                Ask::Start { key, .. } => {
                    starts += 1;
                    cache.finish(&key, Ok(Some("Explain".into())));
                }
                Ask::Wait | Ask::Ready(_) => {}
            }
        }
        assert_eq!(starts, 1);
        // The same text cache-only and live are two lookups.
        let mut backend = Backend::new();
        backend.lookup("Giải thích", true);
        backend.lookup("Giải thích", false);
        backend.lookup("Giải thích", true);
        assert_eq!(backend.calls.len(), 2);
        assert_ne!(cache_key("a", true), cache_key("a", false));
    }

    #[test]
    fn note_label_rule() {
        // Vietnamese source text: the answer is its English translation.
        assert_eq!(
            note_from_answer("Giải thích", Some("Explain".into())),
            note(Label::En, "Explain")
        );
        // English source text: the answer is the original draft.
        assert_eq!(
            note_from_answer("Explain", Some("Giải thích".into())),
            note(Label::ViOriginal, "Giải thích")
        );
        assert_eq!(
            note_from_answer("Fix the bug.", Some("fix teh bug".into())),
            note(Label::EnOriginal, "fix teh bug")
        );
        assert_eq!(note_from_answer("Explain", None), None);
        assert_eq!(note_from_answer("Giải thích", None), None);
        assert_eq!(Label::ViOriginal.text(), "VI gốc");
        assert_eq!(Label::EnOriginal.text(), "EN gốc");
    }

    #[test]
    fn missing_timestamp_is_cache_only() {
        let since = chrono::DateTime::parse_from_rfc3339("2026-10-17T10:00:00Z")
            .unwrap()
            .timestamp_millis();
        assert!(is_cache_only(None, since));
        assert!(is_cache_only(Some("not a time"), since));
        assert!(is_cache_only(Some("2026-10-17T09:59:59.999Z"), since));
        assert!(!is_cache_only(Some("2026-10-17T10:00:00Z"), since));
        assert!(!is_cache_only(Some("2026-10-17T10:00:01+00:00"), since));
    }
}
