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

#[cfg(test)]
mod tests {
    use super::toggle;

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
