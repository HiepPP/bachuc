//! The read-only rules of TASK-271: a provider subagent's running state, and the controls a
//! read-only view hides.

use bachuc_timeline::{shows_fork, shows_permissions, shows_rewind, subagent_turn_active};

#[test]
fn a_child_runs_only_while_its_row_says_running() {
    assert!(subagent_turn_active(Some("running")));
    assert!(!subagent_turn_active(Some("completed")));
    assert!(!subagent_turn_active(Some("failed")));
    assert!(!subagent_turn_active(None), "a missing row is finished");
}

#[test]
fn read_only_hides_the_actions_that_need_a_real_agent() {
    assert!(!shows_fork(true));
    assert!(!shows_rewind(true));
    assert!(!shows_permissions(true));
    assert!(shows_fork(false));
    assert!(shows_rewind(false));
    assert!(shows_permissions(false));
}
