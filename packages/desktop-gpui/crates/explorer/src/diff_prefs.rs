//! The diff choices that the Changes header and every all-files diff of a folder share: the
//! diff mode (`working-diff-comparison/state.ts`), the wrap and whitespace toggles of the
//! Electron `@paseo:changes-preferences`, and the focus request of a Changes click
//! (`focusRequestId`).
//!
//! The model holds no GPUI state; `DiffPrefs` is a GPUI global only so views can observe it.
//! The toggles live in memory for the app session: this crate has no settings file.

use bachuc_client::{DiffCompare, DiffMode};

/// A mode picked in a menu, kept with the dirty state at that time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeOverride {
    pub mode: DiffMode,
    pub dirty_at_selection: bool,
}

/// `resolveWorkingDiffComparisonFromState`: the override while the dirty state is the same as
/// at selection, else `Uncommitted` for a dirty tree and `Base` for a clean one.
pub fn resolve_mode(mode_override: Option<ModeOverride>, dirty: bool) -> DiffMode {
    match mode_override {
        Some(chosen) if chosen.dirty_at_selection == dirty => chosen.mode,
        _ if dirty => DiffMode::Uncommitted,
        _ => DiffMode::Base,
    }
}

/// The request of a mode. `Base` sends the base ref of the loaded commit list when there is one.
pub fn compare_for(mode: DiffMode, base_ref: Option<&str>, ignore_whitespace: bool) -> DiffCompare {
    DiffCompare {
        mode,
        base_ref: match mode {
            DiffMode::Base => base_ref
                .map(str::trim)
                .filter(|base| !base.is_empty())
                .map(str::to_string),
            DiffMode::Uncommitted => None,
        },
        ignore_whitespace,
    }
}

/// `computeBaseRefLabel` without the fallback: `refs/heads/`, `refs/remotes/`, and `origin/`
/// dropped; none for an empty ref.
pub fn ref_label(name: Option<&str>) -> Option<String> {
    let name = name?.trim();
    let name = name
        .strip_prefix("refs/heads/")
        .or_else(|| name.strip_prefix("refs/remotes/"))
        .unwrap_or(name)
        .trim();
    let name = name.strip_prefix("origin/").unwrap_or(name);
    (!name.is_empty()).then(|| name.to_string())
}

/// `computeCommittedDiffDescription`: `<branch> -> <base>`; none when either is missing or both
/// are the same.
pub fn committed_description(branch: Option<&str>, base_ref: Option<&str>) -> Option<String> {
    let branch = ref_label(branch)?;
    let base = ref_label(base_ref)?;
    (branch != base).then(|| format!("{branch} -> {base}"))
}

/// What one folder's views know about its checkout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckoutPrefs {
    /// `CheckoutDetails.files` is not empty; none until the first status answer.
    pub dirty: Option<bool>,
    /// The branch, none for a detached HEAD.
    pub branch: Option<String>,
    /// `CommitList.base_ref` of the last commit list.
    pub base_ref: Option<String>,
    pub mode_override: Option<ModeOverride>,
}

impl CheckoutPrefs {
    /// The resolved mode. A folder with no status yet counts as dirty, as the view asked for the
    /// uncommitted diff before modes existed.
    pub fn mode(&self) -> DiffMode {
        resolve_mode(self.mode_override, self.dirty.unwrap_or(true))
    }
}

/// The last Changes click: which folder and file, and a revision that grows with each click.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusRequest {
    pub cwd: String,
    pub path: String,
    pub revision: u64,
}

#[derive(Clone, Debug, Default)]
pub struct DiffPrefs {
    /// Per folder; a short list, so a vector is cheaper than a map.
    checkouts: Vec<(String, CheckoutPrefs)>,
    /// Wrap long diff lines instead of scrolling them sideways.
    pub wrap_lines: bool,
    /// Ask the daemon to ignore whitespace (`DiffCompare::ignore_whitespace`).
    pub hide_whitespace: bool,
    focus: Option<FocusRequest>,
    focus_revision: u64,
}

impl gpui_kit::Global for DiffPrefs {}

impl DiffPrefs {
    pub fn checkout(&self, cwd: &str) -> CheckoutPrefs {
        self.checkouts
            .iter()
            .find(|(key, _)| key == cwd)
            .map(|(_, prefs)| prefs.clone())
            .unwrap_or_default()
    }

    fn checkout_mut(&mut self, cwd: &str) -> &mut CheckoutPrefs {
        let index = match self.checkouts.iter().position(|(key, _)| key == cwd) {
            Some(index) => index,
            None => {
                self.checkouts
                    .push((cwd.to_string(), CheckoutPrefs::default()));
                self.checkouts.len() - 1
            }
        };
        &mut self.checkouts[index].1
    }

    pub fn mode(&self, cwd: &str) -> DiffMode {
        self.checkout(cwd).mode()
    }

    /// The request an all-files view of `cwd` sends now.
    pub fn compare(&self, cwd: &str) -> DiffCompare {
        let checkout = self.checkout(cwd);
        compare_for(
            checkout.mode(),
            checkout.base_ref.as_deref(),
            self.hide_whitespace,
        )
    }

    /// The diff the Changes tree of `cwd` lists: none in `Uncommitted`, where the status answer
    /// lists the files, else the `Base` request.
    pub fn tree_compare(&self, cwd: &str) -> Option<DiffCompare> {
        (self.mode(cwd) == DiffMode::Base).then(|| self.compare(cwd))
    }

    /// The committed item description of `cwd`.
    pub fn committed_description(&self, cwd: &str) -> Option<String> {
        let checkout = self.checkout(cwd);
        committed_description(checkout.branch.as_deref(), checkout.base_ref.as_deref())
    }

    /// Keeps `mode` for `cwd` until its dirty state changes.
    pub fn select_mode(&mut self, cwd: &str, mode: DiffMode) {
        let checkout = self.checkout_mut(cwd);
        checkout.mode_override = Some(ModeOverride {
            mode,
            dirty_at_selection: checkout.dirty.unwrap_or(true),
        });
    }

    /// Records a status answer. A change of the dirty state drops the override, as
    /// `expireWorkingDiffComparisonsInState` does. False when nothing changed.
    pub fn set_status(&mut self, cwd: &str, dirty: bool, branch: Option<String>) -> bool {
        let checkout = self.checkout_mut(cwd);
        let before = checkout.clone();
        checkout.dirty = Some(dirty);
        checkout.branch = branch;
        if checkout
            .mode_override
            .is_some_and(|chosen| chosen.dirty_at_selection != dirty)
        {
            checkout.mode_override = None;
        }
        *checkout != before
    }

    /// Records the base ref of a commit list answer. False when it did not change.
    pub fn set_base_ref(&mut self, cwd: &str, base_ref: Option<String>) -> bool {
        let checkout = self.checkout_mut(cwd);
        let changed = checkout.base_ref != base_ref;
        checkout.base_ref = base_ref;
        changed
    }

    /// Asks the all-files view of `cwd` to show `path`, and returns the new revision.
    pub fn request_focus(&mut self, cwd: &str, path: &str) -> u64 {
        self.focus_revision += 1;
        self.focus = Some(FocusRequest {
            cwd: cwd.to_string(),
            path: path.to_string(),
            revision: self.focus_revision,
        });
        self.focus_revision
    }

    pub fn focus(&self) -> Option<&FocusRequest> {
        self.focus.as_ref()
    }

    pub fn focus_revision(&self) -> u64 {
        self.focus_revision
    }
}

/// What one all-files view has done with focus requests. A view starts at the revision current
/// at its creation, so the click that created it does not scroll it twice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusTracker {
    seen: u64,
    pending: Option<String>,
}

impl FocusTracker {
    /// `initial` is the focus path the view was created with.
    pub fn new(current_revision: u64, initial: Option<String>) -> FocusTracker {
        FocusTracker {
            seen: current_revision,
            pending: initial,
        }
    }

    /// Takes a request for `cwd` with a revision the view has not seen. True when it is new.
    pub fn observe(&mut self, request: Option<&FocusRequest>, cwd: &str) -> bool {
        match request {
            Some(request) if request.cwd == cwd && request.revision > self.seen => {
                self.seen = request.revision;
                self.pending = Some(request.path.clone());
                true
            }
            _ => false,
        }
    }

    /// The path to scroll to, once the view has a diff. A request before the first answer waits.
    pub fn take(&mut self, loaded: bool) -> Option<String> {
        if loaded { self.pending.take() } else { None }
    }
}

/// The horizontal offset of a file's code after a sideways scroll by `delta` pixels, clamped to
/// `0..=(content_width - code_width)`. A file whose code fits never moves.
pub fn clamp_offset(offset: f32, delta: f32, content_width: f32, code_width: f32) -> f32 {
    let max = (content_width - code_width).max(0.);
    (offset + delta).clamp(0., max)
}

/// The width of a file's code (`columns` characters of `advance` pixels plus `paddings`), and the
/// width a list of `pane` pixels leaves for code beside the gutter. The caller passes the pane
/// width it read outside the list, because a row callback must not read the `ListState`.
pub fn code_widths(
    columns: usize,
    advance: f32,
    paddings: f32,
    pane: f32,
    gutter: f32,
) -> (f32, f32) {
    (paddings + columns as f32 * advance, pane - gutter)
}

/// The fixed row height of the diff list: one height while lines scroll sideways, none while
/// they wrap and each row has its own height.
pub fn uniform_row_height(wrap_lines: bool, line_height: f32) -> Option<f32> {
    (!wrap_lines).then_some(line_height)
}
