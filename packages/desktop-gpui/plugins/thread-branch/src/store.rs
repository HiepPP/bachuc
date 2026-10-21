//! `Branches`: the `BranchInfo` of each folder a branch pill asked for, read through
//! `thread-branch.get`. One store per activation; only the contribution closures hold it, so
//! deactivating the plugin drops it with its poll task.
//!
//! A pill's `state` closure gets only `&App`, so the store keeps the answers in a `RefCell`
//! and starts a read the first time a folder is asked for. Every `POLL` the store reads again
//! each asked folder that an agent or a draft still uses, one read per folder (`pills.ts:166`).
//! It redraws the pills only when a folder's info changed (`signature`, `pills.ts:28`).

use crate::branch::{BranchInfo, MenuAction, copy_text, menu_url, pull_blocked};
use bachuc_client::{Client, GitAction};
use bachuc_plugin_host::{ComposerDraft, ComposerTarget, MessageLevel, PluginContext};
use gpui_kit::{App, ClipboardItem, Entity, SharedString, Task, WeakEntity};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Duration;

/// How often the asked folders are read again (`POLL_MS`).
pub(crate) const POLL: Duration = Duration::from_secs(15);
const GET: &str = "thread-branch.get";
const NO_CONNECTION: &str = "No host connection.";
const NO_PROJECT: &str = "Choose a project first.";
const NO_PR: &str = "No pull request for this branch.";
const NO_REMOTE: &str = "No remote repository for this directory.";

/// How a read asks the server: `force` skips its cache, `fetch` also runs `git fetch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Plain,
    Force,
    Fetch,
}

/// The `thread-branch.get` input (`refreshCwd`, `pills.ts:131`).
pub(crate) fn read_input(cwd: &str, mode: Mode) -> Value {
    match mode {
        Mode::Plain => json!({ "cwd": cwd }),
        Mode::Force => json!({ "cwd": cwd, "force": true }),
        Mode::Fetch => json!({ "cwd": cwd, "fetch": true }),
    }
}

/// The folder an agent's pills read: an agent with no workspace or no folder gets none
/// (`track`, `pills.ts:166`).
pub(crate) fn tracked_cwd<'a>(workspace_id: Option<&str>, cwd: &'a str) -> Option<&'a str> {
    workspace_id
        .filter(|id| !id.is_empty())
        .and(Some(cwd).filter(|cwd| !cwd.is_empty()))
}

/// The folders that something still uses: each agent's and each draft's. A set, so two agents
/// in one folder count once.
pub(crate) fn live_cwds<'a>(
    agents: impl IntoIterator<Item = (Option<&'a str>, &'a str)>,
    drafts: impl IntoIterator<Item = Option<&'a str>>,
) -> BTreeSet<String> {
    agents
        .into_iter()
        .filter_map(|(workspace_id, cwd)| tracked_cwd(workspace_id, cwd))
        .chain(drafts.into_iter().flatten().filter(|cwd| !cwd.is_empty()))
        .map(str::to_string)
        .collect()
}

/// The answers and bookkeeping, without any `App`.
#[derive(Default)]
pub(crate) struct State {
    infos: BTreeMap<String, BranchInfo>,
    in_flight: BTreeSet<String>,
    /// Folders where a branch pill has shown (Q-099): the only ones the poll reads.
    asked: BTreeSet<String>,
    /// Folders whose first read failed. Pills draw often, so they wait for the next tick
    /// instead of asking the server on every draw.
    failed: BTreeSet<String>,
    /// Folders where a Pull press runs. A second press for the same folder does nothing.
    pulling: BTreeSet<String>,
}

impl State {
    /// A Pull press for `cwd`. True when the caller runs the pull: none runs for the folder.
    pub(crate) fn begin_pull(&mut self, cwd: &str) -> bool {
        self.pulling.insert(cwd.to_string())
    }

    pub(crate) fn end_pull(&mut self, cwd: &str) {
        self.pulling.remove(cwd);
    }

    /// A pill asks for `cwd`. True when the caller starts the first read: the folder has no
    /// answer, no read runs for it, and its last first read did not just fail.
    pub(crate) fn ask(&mut self, cwd: &str) -> bool {
        self.asked.insert(cwd.to_string());
        !self.infos.contains_key(cwd)
            && !self.failed.contains(cwd)
            && self.in_flight.insert(cwd.to_string())
    }

    pub(crate) fn info(&self, cwd: &str) -> Option<&BranchInfo> {
        self.infos.get(cwd)
    }

    /// The folders to read on a tick: asked, still live, and not already read. They count as
    /// in flight until `apply` or `failed`.
    pub(crate) fn due(&mut self, live: &BTreeSet<String>) -> Vec<String> {
        self.failed.clear();
        self.asked.retain(|cwd| live.contains(cwd));
        let due: Vec<String> = self
            .asked
            .iter()
            .filter(|cwd| !self.in_flight.contains(*cwd))
            .cloned()
            .collect();
        self.in_flight.extend(due.iter().cloned());
        due
    }

    /// Keeps an answer. True when it differs from the last one, which is when pills redraw.
    pub(crate) fn apply(&mut self, cwd: &str, info: BranchInfo) -> bool {
        self.in_flight.remove(cwd);
        if self.infos.get(cwd) == Some(&info) {
            return false;
        }
        self.infos.insert(cwd.to_string(), info);
        true
    }

    /// A failed read keeps the last info; the next tick tries again.
    pub(crate) fn failed(&mut self, cwd: &str) {
        self.in_flight.remove(cwd);
        self.failed.insert(cwd.to_string());
    }
}

pub(crate) struct Branches {
    ctx: PluginContext,
    state: RefCell<State>,
    drafts: RefCell<Vec<WeakEntity<ComposerDraft>>>,
    /// The tick loop. It holds the store weakly, so dropping the store ends it.
    poll: RefCell<Option<Task<()>>>,
}

impl Branches {
    pub(crate) fn new(ctx: PluginContext) -> Rc<Self> {
        Rc::new(Self {
            ctx,
            state: RefCell::default(),
            drafts: RefCell::default(),
            poll: RefCell::default(),
        })
    }

    pub(crate) fn info(&self, cwd: &str) -> Option<BranchInfo> {
        self.state.borrow().info(cwd).cloned()
    }

    /// The last info of `cwd`, marking the folder as one a pill shows. A missing answer starts
    /// a read, and the first ask starts the poll.
    pub(crate) fn ask(self: &Rc<Self>, cwd: &str, cx: &App) -> Option<BranchInfo> {
        let start = self.state.borrow_mut().ask(cwd);
        if start {
            self.spawn_load(cwd.to_string(), Mode::Plain, false, cx);
        }
        self.ensure_poll(cx);
        self.info(cwd)
    }

    /// The folder of the agent that a pill belongs to.
    pub(crate) fn agent_cwd(&self, target: &ComposerTarget, cx: &App) -> Option<String> {
        let agent_id = target.agent_id.as_deref()?;
        let client: Entity<Client> = self.ctx.client(cx)?;
        client
            .read(cx)
            .agents()
            .iter()
            .find(|agent| agent.id == agent_id)
            .and_then(|agent| tracked_cwd(agent.workspace_id.as_deref(), &agent.cwd))
            .map(str::to_string)
    }

    /// The project folder that the draft composer has selected.
    pub(crate) fn draft_cwd(target: &ComposerTarget, cx: &App) -> Option<String> {
        target
            .draft
            .as_ref()?
            .read(cx)
            .cwd
            .clone()
            .filter(|cwd| !cwd.is_empty())
    }

    /// Keeps a draft in the poll while it exists.
    pub(crate) fn track_draft(&self, draft: &Entity<ComposerDraft>) {
        let mut drafts = self.drafts.borrow_mut();
        drafts.retain(|draft| draft.upgrade().is_some());
        if !drafts
            .iter()
            .any(|known| known.entity_id() == draft.entity_id())
        {
            drafts.push(draft.downgrade());
        }
    }

    fn live(&self, cx: &App) -> BTreeSet<String> {
        let client = self.ctx.client(cx);
        let agents: &[bachuc_client::AgentSummary] = match &client {
            Some(client) => client.read(cx).agents(),
            None => &[],
        };
        let drafts: Vec<Option<String>> = self
            .drafts
            .borrow()
            .iter()
            .filter_map(WeakEntity::upgrade)
            .map(|draft| draft.read(cx).cwd.clone())
            .collect();
        live_cwds(
            agents
                .iter()
                .map(|agent| (agent.workspace_id.as_deref(), agent.cwd.as_str())),
            drafts.iter().map(Option::as_deref),
        )
    }

    fn ensure_poll(self: &Rc<Self>, cx: &App) {
        if self.poll.borrow().is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        let task = cx.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                let Some(store) = weak.upgrade() else { break };
                cx.update(|cx| store.tick(cx));
            }
        });
        *self.poll.borrow_mut() = Some(task);
    }

    /// One poll tick: a read for each asked folder that is still in use.
    fn tick(self: &Rc<Self>, cx: &mut App) {
        let live = self.live(cx);
        let due = self.state.borrow_mut().due(&live);
        for cwd in due {
            self.spawn_load(cwd, Mode::Plain, false, cx);
        }
    }

    /// Reads `cwd`, keeps the answer, and redraws the pills when it changed.
    fn load(
        self: &Rc<Self>,
        cwd: String,
        mode: Mode,
        cx: &mut App,
    ) -> Task<anyhow::Result<BranchInfo>> {
        let answer = self
            .ctx
            .rpc_as::<_, BranchInfo>(GET, &read_input(&cwd, mode), cx);
        let weak = Rc::downgrade(self);
        let ctx = self.ctx.clone();
        cx.spawn(async move |cx| {
            let result = answer.await;
            cx.update(|cx| {
                let Some(store) = weak.upgrade() else { return };
                match &result {
                    Ok(info) => {
                        if store.state.borrow_mut().apply(&cwd, info.clone()) {
                            ctx.notify(cx);
                        }
                    }
                    Err(_) => store.state.borrow_mut().failed(&cwd),
                }
            });
            result
        })
    }

    /// `load` from a context that holds only `&App`. With `report`, a failure shows as an
    /// error message, as Electron toasts a failed pill action.
    fn spawn_load(self: &Rc<Self>, cwd: String, mode: Mode, report: bool, cx: &App) {
        let weak = Rc::downgrade(self);
        let ctx = self.ctx.clone();
        cx.spawn(async move |cx| {
            let Some(task) = cx.update(|cx| weak.upgrade().map(|store| store.load(cwd, mode, cx)))
            else {
                return;
            };
            if let Err(error) = task.await {
                if report {
                    cx.update(|cx| error_message(&ctx, format!("{error:#}"), cx));
                } else {
                    log::debug!("thread-branch.get failed: {error:#}");
                }
            }
        })
        .detach();
    }

    fn error(&self, message: impl Into<SharedString>, cx: &mut App) {
        error_message(&self.ctx, message, cx);
    }

    fn write_branch(&self, info: &BranchInfo, draft: bool, cx: &mut App) {
        match copy_text(info, draft) {
            Ok(branch) => cx.write_to_clipboard(ClipboardItem::new_string(branch)),
            Err(message) => self.error(message, cx),
        }
    }

    /// A menu row press. `draft` selects the New workspace pill, whose folder comes from the
    /// draft and whose copy reads the folder first when it has no answer.
    pub(crate) fn run_menu(
        self: &Rc<Self>,
        action: MenuAction,
        target: &ComposerTarget,
        cx: &mut App,
    ) {
        let draft = target.draft.is_some();
        let cwd = if draft {
            Self::draft_cwd(target, cx)
        } else {
            self.agent_cwd(target, cx)
        };
        let Some(cwd) = cwd else {
            if draft {
                self.error(NO_PROJECT, cx);
            }
            return;
        };
        match action {
            MenuAction::Copy => match self.info(&cwd) {
                Some(info) => self.write_branch(&info, draft, cx),
                None if draft => {
                    let task = self.load(cwd, Mode::Plain, cx);
                    let store = self.clone();
                    cx.spawn(async move |cx| match task.await {
                        Ok(info) => cx.update(|cx| store.write_branch(&info, true, cx)),
                        Err(error) => cx.update(|cx| store.error(format!("{error:#}"), cx)),
                    })
                    .detach();
                }
                None => self.error("No branch name to copy.", cx),
            },
            MenuAction::OpenRepo | MenuAction::OpenIssues | MenuAction::OpenPulls => {
                match self.info(&cwd).and_then(|info| info.remote_url) {
                    Some(url) => cx.open_url(&menu_url(&url, action)),
                    None => self.error(NO_REMOTE, cx),
                }
            }
            MenuAction::Fetch => self.spawn_load(cwd, Mode::Fetch, true, cx),
            // The New workspace menu has no Pull row.
            MenuAction::Pull if draft => {}
            MenuAction::Pull => self.pull(cwd, cx),
            MenuAction::Refresh => self.spawn_load(cwd, Mode::Force, true, cx),
        }
    }

    /// A Pull press on an agent pill: reads `cwd` fresh, refuses a folder with local changes,
    /// runs the daemon pull, and reads the folder again so the pill drops its `↓` count.
    fn pull(self: &Rc<Self>, cwd: String, cx: &mut App) {
        if !self.state.borrow_mut().begin_pull(&cwd) {
            return;
        }
        let Some(client) = self.ctx.client(cx) else {
            self.state.borrow_mut().end_pull(&cwd);
            self.error(NO_CONNECTION, cx);
            return;
        };
        let fresh = self.load(cwd.clone(), Mode::Force, cx);
        let store = self.clone();
        cx.spawn(async move |cx| {
            let outcome = async {
                let info = fresh.await.map_err(|error| format!("{error:#}"))?;
                if let Some(message) = pull_blocked(&info) {
                    return Err(message.to_string());
                }
                cx.update(|cx| {
                    client.update(cx, |client, cx| {
                        client.git_action(&cwd, GitAction::Pull, cx)
                    })
                })
                .await
                .map_err(|error| format!("{error:#}"))?;
                cx.update(|cx| store.load(cwd.clone(), Mode::Force, cx))
                    .await
                    .map_err(|error| format!("{error:#}"))?;
                Ok::<(), String>(())
            }
            .await;
            cx.update(|cx| {
                store.state.borrow_mut().end_pull(&cwd);
                if let Err(message) = outcome {
                    store.error(message, cx);
                }
            });
        })
        .detach();
    }

    /// A PR pill press: opens the pull request of the agent's folder.
    pub(crate) fn open_pr(&self, target: &ComposerTarget, cx: &mut App) {
        let url = self
            .agent_cwd(target, cx)
            .and_then(|cwd| self.info(&cwd))
            .and_then(|info| info.pr)
            .map(|pr| pr.url);
        match url {
            Some(url) => cx.open_url(&url),
            None => self.error(NO_PR, cx),
        }
    }

    /// A changes pill press: reads the agent's folder again, skipping the server cache.
    pub(crate) fn refresh_agent(self: &Rc<Self>, target: &ComposerTarget, cx: &mut App) {
        if let Some(cwd) = self.agent_cwd(target, cx) {
            self.spawn_load(cwd, Mode::Force, true, cx);
        }
    }
}

fn error_message(ctx: &PluginContext, message: impl Into<SharedString>, cx: &mut App) {
    ctx.show_message(message.into(), MessageLevel::Error, cx);
}

#[cfg(test)]
mod tests {
    use super::{Mode, State, live_cwds, read_input, tracked_cwd};
    use crate::branch::BranchInfo;
    use serde_json::json;
    use std::collections::BTreeSet;

    fn info(branch: &str) -> BranchInfo {
        BranchInfo {
            repo: true,
            branch: Some(branch.into()),
            detached: false,
            sha: Some("a1b2c3d".into()),
            dirty: false,
            changes: None,
            upstream: None,
            ahead: None,
            behind: None,
            pr: None,
            remote_url: None,
        }
    }

    #[test]
    fn reads_send_the_inputs_of_pills_ts() {
        assert_eq!(read_input("/r", Mode::Plain), json!({"cwd": "/r"}));
        assert_eq!(
            read_input("/r", Mode::Force),
            json!({"cwd": "/r", "force": true})
        );
        assert_eq!(
            read_input("/r", Mode::Fetch),
            json!({"cwd": "/r", "fetch": true})
        );
    }

    #[test]
    fn an_agent_with_no_workspace_or_no_folder_gets_no_pills() {
        assert_eq!(tracked_cwd(Some("w1"), "/r"), Some("/r"));
        assert_eq!(tracked_cwd(None, "/r"), None);
        assert_eq!(tracked_cwd(Some(""), "/r"), None);
        assert_eq!(tracked_cwd(Some("w1"), ""), None);
    }

    #[test]
    fn poll_one_read_per_cwd() {
        // Two agents and a draft share /repo; one agent has no workspace; /other has no pill.
        let live = live_cwds(
            [
                (Some("w1"), "/repo"),
                (Some("w1"), "/repo"),
                (None, "/lonely"),
                (Some("w2"), "/other"),
            ],
            [Some("/repo"), None, Some("")],
        );
        assert_eq!(
            live,
            BTreeSet::from(["/repo".to_string(), "/other".to_string()])
        );
        let mut state = State::default();
        // /repo is asked by two agents and a draft; /lonely never shows a pill.
        assert!(state.ask("/repo"));
        assert!(!state.ask("/repo"));
        state.asked.insert("/gone".into());
        // The first read still runs, so the first tick reads nothing for /repo.
        assert!(state.due(&live).is_empty());
        assert!(state.apply("/repo", info("main")));
        // Now one read for /repo; the pruned /gone and unasked /other get none.
        assert_eq!(state.due(&live), vec!["/repo".to_string()]);
        assert!(!state.asked.contains("/gone"));
        // The next tick comes before the answer: no second read for the same folder.
        assert!(state.due(&live).is_empty());
        state.failed("/repo");
        assert_eq!(state.due(&live), vec!["/repo".to_string()]);
    }

    #[test]
    fn a_folder_is_read_for_the_first_time_once() {
        let mut state = State::default();
        assert!(state.ask("/repo"));
        // Several pills ask while the read runs.
        assert!(!state.ask("/repo"));
        assert!(!state.ask("/repo"));
        state.apply("/repo", info("main"));
        assert!(!state.ask("/repo"));
        assert_eq!(state.info("/repo").unwrap().branch.as_deref(), Some("main"));
    }

    #[test]
    fn a_failed_first_read_waits_for_the_next_tick() {
        let mut state = State::default();
        let live = BTreeSet::from(["/repo".to_string()]);
        assert!(state.ask("/repo"));
        state.failed("/repo");
        // Pills draw again at once; they do not ask the server again.
        assert!(!state.ask("/repo"));
        assert!(!state.ask("/repo"));
        // The tick reads it, and a success ends the wait.
        assert_eq!(state.due(&live), vec!["/repo".to_string()]);
        assert!(state.apply("/repo", info("main")));
        assert!(!state.ask("/repo"));
    }

    #[test]
    fn one_pull_at_a_time_per_folder() {
        let mut state = State::default();
        assert!(state.begin_pull("/repo"));
        assert!(!state.begin_pull("/repo"));
        assert!(state.begin_pull("/other"));
        state.end_pull("/repo");
        assert!(state.begin_pull("/repo"));
    }

    #[test]
    fn same_signature_no_notify() {
        let mut state = State::default();
        assert!(state.apply("/repo", info("main")));
        // The same answer again: no redraw.
        assert!(!state.apply("/repo", info("main")));
        // A changed field redraws.
        assert!(state.apply("/repo", info("feat/x")));
        let mut dirty = info("feat/x");
        dirty.dirty = true;
        assert!(state.apply("/repo", dirty));
        // Another folder has its own signature.
        assert!(state.apply("/other", info("feat/x")));
        // A failed read keeps the last info and redraws nothing.
        state.failed("/repo");
        assert!(state.info("/repo").unwrap().dirty);
    }
}
