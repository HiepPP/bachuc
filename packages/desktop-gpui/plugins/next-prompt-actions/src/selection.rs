//! Pure selection rules for structured suggestion blocks (`shared/selection.ts`).

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Choice { index: usize, ids: Vec<String> },
    Together { set: usize, ids: Vec<String> },
    FollowUp { ids: Vec<String> },
    Alone { ids: Vec<String> },
}

impl Group {
    pub(crate) fn ids(&self) -> &[String] {
        match self {
            Self::Choice { ids, .. }
            | Self::Together { ids, .. }
            | Self::FollowUp { ids }
            | Self::Alone { ids } => ids,
        }
    }
}

fn holds(list: &[String], id: &str) -> bool {
    list.iter().any(|item| item == id)
}

/// Exclusive groups become radio groups. Other suggestions linked by allowed combinations
/// share a group: "follow-up" when a linked combination includes an exclusive choice, else
/// "together". Suggestions in no combination are grouped as "alone".
pub(crate) fn layout(
    order: &[String],
    groups: &[Vec<String>],
    combos: &[Vec<String>],
) -> Vec<Group> {
    let exclusive = |id: &str| groups.iter().any(|group| holds(group, id));
    let combined = |id: &str| combos.iter().any(|combo| holds(combo, id));
    let mut result: Vec<Group> = groups
        .iter()
        .enumerate()
        .map(|(index, ids)| Group::Choice {
            index,
            ids: ids.clone(),
        })
        .collect();
    let mut seen: Vec<String> = Vec::new();
    let mut together = Vec::new();
    for id in order {
        if exclusive(id) || holds(&seen, id) || !combined(id) {
            continue;
        }
        let mut ids = vec![id.clone()];
        seen.push(id.clone());
        let mut at = 0;
        while at < ids.len() {
            let current = ids[at].clone();
            for combo in combos.iter().filter(|combo| holds(combo, &current)) {
                for other in combo {
                    if !exclusive(other) && !holds(&seen, other) {
                        seen.push(other.clone());
                        ids.push(other.clone());
                    }
                }
            }
            at += 1;
        }
        let follow = combos.iter().any(|combo| {
            combo.iter().any(|x| holds(&ids, x)) && combo.iter().any(|x| exclusive(x))
        });
        if follow {
            result.push(Group::FollowUp { ids });
        } else {
            together.push(result.len());
            result.push(Group::Together { set: 0, ids });
        }
    }
    if together.len() > 1 {
        for (number, &at) in together.iter().enumerate() {
            if let Group::Together { set, .. } = &mut result[at] {
                *set = number + 1;
            }
        }
    }
    let alone: Vec<String> = order
        .iter()
        .filter(|id| !exclusive(id) && !combined(id))
        .cloned()
        .collect();
    if !alone.is_empty() {
        result.push(Group::Alone { ids: alone });
    }
    result
}

/// A radio replaces its exclusive group's selection; the result must fit one declared
/// combination.
pub(crate) fn reachable(
    selected: &[String],
    id: &str,
    groups: &[Vec<String>],
    combos: &[Vec<String>],
) -> bool {
    let group = groups
        .iter()
        .find(|group| holds(group, id))
        .map_or(&[][..], Vec::as_slice);
    let mut next: Vec<&str> = selected
        .iter()
        .filter(|x| !holds(group, x))
        .map(String::as_str)
        .collect();
    next.push(id);
    next.len() == 1
        || combos
            .iter()
            .any(|combo| next.iter().all(|x| holds(combo, x)))
}

/// The selection after a press on `id`: off when on, else on with its group's other choice off.
pub(crate) fn toggle(selected: &[String], id: &str, groups: &[Vec<String>]) -> Vec<String> {
    if holds(selected, id) {
        return selected.iter().filter(|x| *x != id).cloned().collect();
    }
    let group = groups
        .iter()
        .find(|group| holds(group, id))
        .map_or(&[][..], Vec::as_slice);
    let mut next: Vec<String> = selected
        .iter()
        .filter(|x| !holds(group, x))
        .cloned()
        .collect();
    next.push(id.to_string());
    next
}

/// The farthest a press may move and still count as a click, in pixels.
const CLICK_SLOP_PX: f32 = 4.;

/// Whether a pick row toggles on release. Text selection runs on a window-level layer that
/// never stops the event, so a drag over the row text still reaches the row's click handler.
pub(crate) fn toggles_on_release(moved_px: f32, has_selection: bool) -> bool {
    moved_px <= CLICK_SLOP_PX && !has_selection
}

/// What a click on a pick row does, from its click count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowClick {
    Toggle,
    /// The second click of a double-click: put back what the first click changed.
    Restore,
    Skip,
}

/// A double-click selects a word, so its second click undoes the toggle of the first click
/// instead of leaving the row changed. Later clicks of a multi-click do nothing, because the
/// second click already restored the row.
pub(crate) fn row_click(click_count: usize, moved_px: f32, has_selection: bool) -> RowClick {
    match click_count {
        2 => RowClick::Restore,
        0 | 1 if toggles_on_release(moved_px, has_selection) => RowClick::Toggle,
        _ => RowClick::Skip,
    }
}

#[cfg(test)]
mod tests {
    use super::toggle;

    #[test]
    fn a_double_click_restores_and_a_triple_click_does_nothing() {
        use super::{RowClick, row_click};
        assert_eq!(row_click(1, 0., false), RowClick::Toggle);
        assert_eq!(row_click(1, 9., false), RowClick::Skip);
        assert_eq!(row_click(1, 0., true), RowClick::Skip);
        assert_eq!(row_click(2, 0., true), RowClick::Restore);
        assert_eq!(row_click(2, 9., true), RowClick::Restore);
        assert_eq!(row_click(3, 0., true), RowClick::Skip);
        assert_eq!(row_click(0, 0., false), RowClick::Toggle);
    }

    #[test]
    fn a_still_click_toggles_and_a_drag_or_selection_does_not() {
        use super::toggles_on_release;
        assert!(toggles_on_release(0., false));
        assert!(toggles_on_release(4., false));
        assert!(!toggles_on_release(4.5, false));
        assert!(!toggles_on_release(0., true));
    }

    #[test]
    fn a_radio_press_replaces_its_group_choice_and_a_second_press_clears_it() {
        let s = |items: &[&str]| -> Vec<String> { items.iter().map(|x| x.to_string()).collect() };
        let groups = vec![s(&["x", "y"])];
        assert_eq!(
            toggle(&s(&["x", "risks"]), "y", &groups),
            s(&["risks", "y"])
        );
        assert_eq!(toggle(&s(&["x", "risks"]), "x", &groups), s(&["risks"]));
    }
}
