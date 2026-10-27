//! The three thread actions of `client/remove.ts` (45-249): Jump To Parent, Remove, and
//! Remove & New Thread. GPUI draws them as one icon group in the workspace header, so they
//! never cover the timeline and sit away from the composer. Each pill finds its run in the
//! store snapshot by the focused agent's `agent_id`; the Electron workspace lookup is not
//! needed.

use crate::model::{ConnectionState, Run};
use crate::store::BoardStore;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    ComposerPill, ComposerTarget, MessageLevel, NewWorkspaceRequest, PillBehavior, PillPlacement,
    PillState, PluginContext, ShowOnDraft,
};
use gpui_kit::{App, Entity, Hsla, rgb};
use std::rc::Rc;

/// Navigation, not removal: orange sets it apart from the Remove actions beside it
/// (`remove.ts:29-30`).
const PARENT_COLOR: u32 = 0xf97316;
/// Red marks the one icon that only removes, as the design picked on 2026-10-28.
const REMOVE_COLOR: u32 = 0x9c3a3a;
const RUN_CHANGED: &str = "Run changed. Refresh and retry.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PillKind {
    Remove,
    RemoveAndNew,
    Parent,
}

/// A store error hides every pill, as Electron clears them when the host cannot confirm
/// finished Board membership (`remove.ts:238`).
pub(crate) fn pills_hidden(connection: ConnectionState) -> bool {
    matches!(
        connection,
        ConnectionState::Error | ConnectionState::Offline
    )
}

/// The run a pill acts on: the composer's agent's run, when the pill applies to it.
pub(crate) fn pill_run<'a>(
    kind: PillKind,
    runs: &'a [Run],
    agent_id: Option<&str>,
    hidden: bool,
) -> Option<&'a Run> {
    if hidden {
        return None;
    }
    let agent_id = agent_id?;
    let run = runs.iter().find(|run| run.agent_id == agent_id)?;
    let applies = match kind {
        PillKind::Remove => !run.running(),
        PillKind::RemoveAndNew => !run.running() && run.cwd.is_some(),
        PillKind::Parent => run
            .parent_agent_id
            .as_deref()
            .is_some_and(|parent| !parent.is_empty() && parent != run.agent_id),
    };
    applies.then_some(run)
}

fn current_run(
    store: &Entity<BoardStore>,
    kind: PillKind,
    target: &ComposerTarget,
    cx: &App,
) -> Option<Run> {
    let store = store.read(cx);
    let hidden = pills_hidden(store.connection(crate::model::text::now_ms()));
    let runs = store.snapshot().map_or(&[][..], |snapshot| &snapshot.runs);
    pill_run(kind, runs, target.agent_id.as_deref(), hidden).cloned()
}

/// Removes the run, then leaves the thread: back to the Board, or to the New workspace screen
/// of its project. A failed close reports itself and never undoes the removal.
fn remove_and_leave(
    store: &Entity<BoardStore>,
    ctx: &PluginContext,
    run: Run,
    new_thread: bool,
    cx: &mut App,
) {
    let task = store.update(cx, |store, cx| store.remove(&run.id, cx));
    let weak = store.downgrade();
    let ctx = ctx.clone();
    cx.spawn(async move |cx| {
        match task.await {
            Ok(true) => {}
            Ok(false) => {
                cx.update(|cx| ctx.show_message(RUN_CHANGED.into(), MessageLevel::Error, cx));
                return;
            }
            Err(error) => {
                cx.update(|cx| {
                    ctx.show_message(format!("{error:#}").into(), MessageLevel::Error, cx)
                });
                return;
            }
        }
        let close = cx.update(|cx| {
            match (new_thread, run.cwd.clone()) {
                (true, Some(cwd)) => ctx.open_new_workspace(
                    NewWorkspaceRequest {
                        cwd,
                        project_id: run.project_id.clone(),
                        name: Some(run.project.clone()),
                    },
                    cx,
                ),
                _ => ctx.open_surface(crate::SURFACE_ID, Default::default(), cx),
            }
            weak.update(cx, |store, cx| store.close_runtime(&run.agent_id, cx))
                .ok()
        });
        if let Some(close) = close
            && let Err(error) = close.await
        {
            cx.update(|cx| ctx.show_message(error.to_string().into(), MessageLevel::Error, cx));
        }
    })
    .detach();
}

/// The pill state of `kind`: its title (the tooltip), icon, and press. Header pills draw no
/// label.
fn pill_state(
    kind: PillKind,
    run: Run,
    store: &Entity<BoardStore>,
    ctx: &PluginContext,
) -> PillState {
    let (title, icon, color) = match kind {
        PillKind::Remove => (
            "Remove from Board",
            IconName::X,
            Some(Hsla::from(rgb(REMOVE_COLOR))),
        ),
        PillKind::RemoveAndNew => ("Remove and Start New Thread", IconName::SquarePen, None),
        PillKind::Parent => (
            "Jump To Parent",
            IconName::CornerLeftUp,
            Some(Hsla::from(rgb(PARENT_COLOR))),
        ),
    };
    let store = store.clone();
    let ctx = ctx.clone();
    PillState {
        title: title.into(),
        label: None,
        icon: Some(icon),
        color,
        disabled: false,
        behavior: PillBehavior::Action {
            on_press: Rc::new(move |_target, _window, cx| match kind {
                PillKind::Parent => {
                    if let Some(parent) = &run.parent_agent_id {
                        ctx.open_agent(parent, cx);
                    }
                }
                PillKind::Remove | PillKind::RemoveAndNew => remove_and_leave(
                    &store,
                    &ctx,
                    run.clone(),
                    kind == PillKind::RemoveAndNew,
                    cx,
                ),
            }),
            new_workspace_panel: None,
        },
    }
}

/// The three header pills, left to right: navigation first, then the two removals.
const HEADER_PILLS: [(&str, PillKind); 3] = [
    ("jump-to-parent", PillKind::Parent),
    ("remove", PillKind::Remove),
    ("remove-new-thread", PillKind::RemoveAndNew),
];

pub(crate) fn header_pills(store: &Entity<BoardStore>, ctx: &PluginContext) -> Vec<ComposerPill> {
    HEADER_PILLS
        .into_iter()
        .map(|(id, kind)| {
            let store = store.clone();
            let ctx = ctx.clone();
            ComposerPill {
                id,
                placement: PillPlacement::Header,
                show_on_draft: ShowOnDraft::No,
                workspace_id: None,
                agent_id: None,
                state: Rc::new(move |target, cx| {
                    let run = current_run(&store, kind, target, cx)?;
                    Some(pill_state(kind, run, &store, &ctx))
                }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RunStatus;
    use crate::model::run::tests::card;

    fn run(agent_id: &str, status: RunStatus) -> Run {
        let mut run = card(agent_id, "app", false);
        run.agent_id = agent_id.into();
        run.status = status;
        run
    }

    #[test]
    fn a_running_run_has_no_remove_and_a_finished_one_has_both() {
        let mut finished = run("done", RunStatus::Completed);
        finished.cwd = Some("/p/app".into());
        let runs = [run("busy", RunStatus::Running), finished];
        assert!(pill_run(PillKind::Remove, &runs, Some("busy"), false).is_none());
        assert!(pill_run(PillKind::RemoveAndNew, &runs, Some("busy"), false).is_none());
        assert_eq!(
            pill_run(PillKind::Remove, &runs, Some("done"), false).map(|run| run.id.as_str()),
            Some("done")
        );
        assert!(pill_run(PillKind::RemoveAndNew, &runs, Some("done"), false).is_some());
        // Failed, cancelled, and unknown runs are finished too.
        let failed = [run("f", RunStatus::Unknown)];
        assert!(pill_run(PillKind::Remove, &failed, Some("f"), false).is_some());
    }

    #[test]
    fn no_cwd_means_no_remove_and_new_thread() {
        let runs = [run("done", RunStatus::Completed)];
        assert!(pill_run(PillKind::Remove, &runs, Some("done"), false).is_some());
        assert!(pill_run(PillKind::RemoveAndNew, &runs, Some("done"), false).is_none());
    }

    #[test]
    fn jump_to_parent_needs_a_parent_other_than_itself() {
        let mut child = run("child", RunStatus::Running);
        child.parent_agent_id = Some("parent".into());
        let mut own = run("own", RunStatus::Completed);
        own.parent_agent_id = Some("own".into());
        let runs = [child, own, run("top", RunStatus::Running)];
        assert!(pill_run(PillKind::Parent, &runs, Some("child"), false).is_some());
        assert!(pill_run(PillKind::Parent, &runs, Some("own"), false).is_none());
        assert!(pill_run(PillKind::Parent, &runs, Some("top"), false).is_none());
    }

    #[test]
    fn a_store_error_or_an_unknown_agent_hides_every_pill() {
        let mut done = run("done", RunStatus::Completed);
        done.cwd = Some("/p".into());
        done.parent_agent_id = Some("parent".into());
        let runs = [done];
        for kind in [PillKind::Remove, PillKind::RemoveAndNew, PillKind::Parent] {
            assert!(pill_run(kind, &runs, Some("done"), false).is_some());
            assert!(pill_run(kind, &runs, Some("done"), true).is_none());
            assert!(pill_run(kind, &runs, Some("other"), false).is_none());
            assert!(pill_run(kind, &runs, None, false).is_none());
        }
        assert!(pills_hidden(ConnectionState::Error));
        assert!(pills_hidden(ConnectionState::Offline));
        assert!(!pills_hidden(ConnectionState::Live));
        assert!(!pills_hidden(ConnectionState::Stale));
    }

    #[test]
    fn the_header_group_reads_parent_remove_then_remove_and_new() {
        let kinds: Vec<_> = HEADER_PILLS.iter().map(|(_, kind)| *kind).collect();
        assert_eq!(
            kinds,
            [PillKind::Parent, PillKind::Remove, PillKind::RemoveAndNew]
        );
    }

    #[test]
    fn the_parent_pill_is_the_orange_of_remove_ts() {
        let orange = Hsla::from(rgb(PARENT_COLOR)).to_rgb();
        assert!((orange.r - 0xf9 as f32 / 255.).abs() < 1e-3);
        assert!((orange.g - 0x73 as f32 / 255.).abs() < 1e-3);
        assert!((orange.b - 0x16 as f32 / 255.).abs() < 1e-3);
    }
}
