//! `removeFinishedRun` and the close answer of `closeRemovedRuntimes` (`client/remove.ts`).

use super::run::{BoardSnapshot, RunStatus};
use serde::{Deserialize, Serialize};
use std::future::Future;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoveInput<'a> {
    pub id: &'a str,
    pub observing_since: &'a str,
    pub ended_at: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RemoveOutput {
    pub removed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CloseOutput {
    #[allow(dead_code)]
    pub closed: u64,
    #[allow(dead_code)]
    pub kept: u64,
    pub failed: u64,
}

/// A failed close names how many processes stayed open.
pub(crate) fn close_result(output: &CloseOutput) -> anyhow::Result<()> {
    match output.failed {
        0 => Ok(()),
        1 => anyhow::bail!("Could not close 1 thread process."),
        failed => anyhow::bail!("Could not close {failed} thread processes."),
    }
}

/// Removes a finished run. When the host refuses, reads the Board once: a run that is gone
/// counts as removed, and the same finished turn under a fresh scope retries once.
pub(crate) async fn remove_finished_run<RemoveFut, ReadFut>(
    id: &str,
    ended_at: Option<&str>,
    observing_since: &str,
    remove: impl Fn(String) -> RemoveFut,
    read: impl FnOnce() -> ReadFut,
) -> anyhow::Result<bool>
where
    RemoveFut: Future<Output = anyhow::Result<bool>>,
    ReadFut: Future<Output = anyhow::Result<BoardSnapshot>>,
{
    if remove(observing_since.to_string()).await? {
        return Ok(true);
    }
    let fresh = read().await?;
    let Some(current) = fresh.runs.iter().find(|run| run.id == id) else {
        return Ok(true);
    };
    if fresh.observing_since == observing_since
        || current.status == RunStatus::Running
        || current.ended_at.as_deref() != ended_at
    {
        return Ok(false);
    }
    remove(fresh.observing_since).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::run::tests::card;
    use std::cell::RefCell;
    use std::future::ready;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    fn block_on<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                return value;
            }
        }
    }

    fn board(scope: &str, ended_at: Option<&str>) -> BoardSnapshot {
        let mut run = card("finished", "p", false);
        run.status = RunStatus::Completed;
        run.ended_at = ended_at.map(str::to_string);
        BoardSnapshot {
            runs: vec![run],
            observing_since: scope.into(),
        }
    }

    /// Runs the rule with a host that removes only under `current`, and returns the result
    /// and the scopes it sent.
    fn run(fresh: anyhow::Result<BoardSnapshot>) -> (bool, Vec<String>) {
        let scopes = RefCell::new(Vec::new());
        let removed = block_on(remove_finished_run(
            "finished",
            Some("end"),
            "old",
            |scope| {
                let removed = scope == "current";
                scopes.borrow_mut().push(scope);
                ready(Ok(removed))
            },
            || ready(fresh),
        ))
        .unwrap();
        (removed, scopes.into_inner())
    }

    #[test]
    fn removed_at_once_is_true() {
        let removed = block_on(remove_finished_run(
            "finished",
            Some("end"),
            "old",
            |_| ready(Ok(true)),
            || async { panic!("no read after a removal") },
        ));
        assert!(removed.unwrap());
    }

    #[test]
    fn a_run_gone_on_re_read_is_true() {
        let gone = BoardSnapshot {
            runs: vec![],
            observing_since: "current".into(),
        };
        assert_eq!(run(Ok(gone)), (true, vec!["old".into()]));
    }

    #[test]
    fn the_same_scope_is_false() {
        assert_eq!(
            run(Ok(board("old", Some("end")))),
            (false, vec!["old".into()])
        );
    }

    #[test]
    fn a_fresh_scope_returns_the_second_call() {
        assert_eq!(
            run(Ok(board("current", Some("end")))),
            (true, vec!["old".into(), "current".into()])
        );
        // A different finished turn never retries.
        assert_eq!(
            run(Ok(board("current", Some("new end")))),
            (false, vec!["old".into()])
        );
    }

    #[test]
    fn a_failed_read_is_an_error_and_a_failed_close_names_the_count() {
        let failed = block_on(remove_finished_run(
            "finished",
            None,
            "old",
            |_| ready(Ok(false)),
            || ready(Err(anyhow::anyhow!("offline"))),
        ));
        assert!(failed.is_err());
        let output = |failed| CloseOutput {
            closed: 1,
            kept: 0,
            failed,
        };
        assert!(close_result(&output(0)).is_ok());
        assert_eq!(
            close_result(&output(1)).unwrap_err().to_string(),
            "Could not close 1 thread process."
        );
        assert_eq!(
            close_result(&output(3)).unwrap_err().to_string(),
            "Could not close 3 thread processes."
        );
    }
}
