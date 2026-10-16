//! The row model of the diff viewer: every file of one `checkout_diff` answer in one scroll, a
//! header per file in the daemon's order, then its lines or its placeholder. A collapsed file
//! keeps only its header, as the Electron diff document does.
//!
//! The model holds no GPUI state. The view draws only the rows in view, so a large diff costs
//! one parse and one row list here.

use crate::changes::Change;
use crate::diff::{NoDiff, ParsedDiff, parse};
use bachuc_client::{FileDiff, FileDiffStatus};

/// What a file shows under its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileBody {
    Lines(ParsedDiff),
    /// The daemon sent no diff text: the file is too large or binary.
    Placeholder(NoDiff),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffFile {
    pub path: String,
    /// The path before a rename.
    pub old_path: Option<String>,
    pub change: Change,
    pub additions: u32,
    pub deletions: u32,
    pub body: FileBody,
}

impl DiffFile {
    pub fn from_diff(diff: FileDiff) -> DiffFile {
        let body = match diff.status {
            FileDiffStatus::Ok => FileBody::Lines(parse(&diff.text)),
            FileDiffStatus::TooLarge => FileBody::Placeholder(NoDiff::TooLarge),
            FileDiffStatus::Binary => FileBody::Placeholder(NoDiff::Binary),
        };
        DiffFile {
            change: Change::of(diff.is_new, diff.is_deleted),
            path: diff.path,
            old_path: diff.old_path.filter(|old| !old.is_empty()),
            additions: diff.additions,
            deletions: diff.deletions,
            body,
        }
    }

    /// `old -> new` for a rename, else the path.
    pub fn display_path(&self) -> String {
        match &self.old_path {
            Some(old) if *old != self.path => format!("{old} -> {}", self.path),
            _ => self.path.clone(),
        }
    }
}

/// One row of the scroll. `file` and `line` index into `DiffDoc::files` and the file's lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffRow {
    Header {
        file: usize,
    },
    Line {
        file: usize,
        line: usize,
    },
    /// The too-large or binary placeholder of a file.
    Placeholder {
        file: usize,
        reason: NoDiff,
    },
}

#[derive(Clone, Debug, Default)]
pub struct DiffDoc {
    files: Vec<DiffFile>,
    collapsed: Vec<bool>,
    rows: Vec<DiffRow>,
    max_number: u32,
    longest: usize,
}

impl DiffDoc {
    pub fn new(diffs: Vec<FileDiff>) -> DiffDoc {
        let files: Vec<DiffFile> = diffs.into_iter().map(DiffFile::from_diff).collect();
        let mut max_number = 0;
        let mut longest = 0;
        for file in &files {
            if let FileBody::Lines(parsed) = &file.body {
                for line in &parsed.lines {
                    max_number = max_number.max(line.number.unwrap_or(0));
                    longest = longest.max(line.text.len());
                }
            }
        }
        let mut doc = DiffDoc {
            collapsed: vec![false; files.len()],
            files,
            rows: Vec::new(),
            max_number,
            longest,
        };
        doc.rebuild();
        doc
    }

    fn rebuild(&mut self) {
        let mut rows = Vec::with_capacity(self.rows.len().max(self.files.len()));
        for (file, entry) in self.files.iter().enumerate() {
            rows.push(DiffRow::Header { file });
            if self.collapsed[file] {
                continue;
            }
            match &entry.body {
                FileBody::Lines(parsed) => {
                    rows.extend((0..parsed.lines.len()).map(|line| DiffRow::Line { file, line }))
                }
                FileBody::Placeholder(reason) => rows.push(DiffRow::Placeholder {
                    file,
                    reason: *reason,
                }),
            }
        }
        self.rows = rows;
    }

    pub fn files(&self) -> &[DiffFile] {
        &self.files
    }

    pub fn rows(&self) -> &[DiffRow] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The header row of the file at `path` (or renamed from it), for the first scroll.
    pub fn header_row(&self, path: &str) -> Option<usize> {
        let file = self
            .files
            .iter()
            .position(|file| file.path == path || file.old_path.as_deref() == Some(path))?;
        self.rows
            .iter()
            .position(|row| matches!(row, DiffRow::Header { file: index } if *index == file))
    }

    pub fn is_collapsed(&self, file: usize) -> bool {
        self.collapsed.get(file).copied().unwrap_or(false)
    }

    pub fn toggle_file(&mut self, file: usize) {
        if let Some(collapsed) = self.collapsed.get_mut(file) {
            *collapsed = !*collapsed;
            self.rebuild();
        }
    }

    pub fn all_collapsed(&self) -> bool {
        !self.collapsed.is_empty() && self.collapsed.iter().all(|collapsed| *collapsed)
    }

    pub fn set_all_collapsed(&mut self, collapsed: bool) {
        self.collapsed
            .iter_mut()
            .for_each(|value| *value = collapsed);
        self.rebuild();
    }

    /// The widest line number of every file, so all gutters line up.
    pub fn max_number(&self) -> u32 {
        self.max_number
    }

    /// The byte length of the longest line, which sizes the horizontal scroll.
    pub fn longest_line(&self) -> usize {
        self.longest
    }

    /// The summed counts of every file.
    pub fn totals(&self) -> (u32, u32) {
        self.files.iter().fold((0, 0), |(added, removed), file| {
            (added + file.additions, removed + file.deletions)
        })
    }
}

/// Request ids for a view that may ask again before an answer arrives: only the answer to the
/// newest request applies.
#[derive(Clone, Copy, Debug, Default)]
pub struct Latest {
    id: u64,
}

impl Latest {
    /// Starts a request and returns its id.
    pub fn begin(&mut self) -> u64 {
        self.id += 1;
        self.id
    }

    /// The id of the newest request.
    pub fn current(&self) -> u64 {
        self.id
    }

    pub fn is_current(&self, id: u64) -> bool {
        id == self.id
    }
}
