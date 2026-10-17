//! `packages/app/src/panels/agent-tracks.tsx`: when the track strip shows. The strip holds, in
//! order, the tasks, the subagents, and the plugin track pills, then the turn metrics and the
//! diff stat at the right edge.

/// `hasAgentTracks`: a leading pill has something to show.
pub fn has_agent_tracks(
    subagents: usize,
    tasks: usize,
    archiving: bool,
    plugin_pills: bool,
) -> bool {
    subagents > 0 || tasks > 0 || archiving || plugin_pills
}

/// The strip shows with a diff stat, turn metrics, or any leading pill (`AgentTracks`).
pub fn strip_visible(diff_stat: bool, turn_metrics: bool, leading: bool) -> bool {
    diff_stat || turn_metrics || leading
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_source_alone_shows_the_leading_pills() {
        assert!(!has_agent_tracks(0, 0, false, false));
        assert!(has_agent_tracks(1, 0, false, false));
        assert!(has_agent_tracks(0, 7, false, false));
        assert!(has_agent_tracks(0, 0, true, false));
        assert!(has_agent_tracks(0, 0, false, true));
    }

    #[test]
    fn nothing_to_show_hides_the_strip() {
        assert!(!strip_visible(false, false, false));
        assert!(strip_visible(true, false, false));
        assert!(strip_visible(false, true, false));
        assert!(strip_visible(false, false, true));
    }
}
