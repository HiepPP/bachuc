//! The rules of `shared/catalog.ts`, `shared/contracts.ts`, `shared/draft.ts`, and
//! `shared/settings.ts`: the catalog, the pill label, and the decoders of the wire and the
//! saved values. Every rule is a pure function.

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

pub(crate) const READ: &str = "skill-pins.read";
pub(crate) const WRITE: &str = "skill-pins.write";

/// One pinnable skill (`catalog` of `catalog.ts`).
pub(crate) struct Skill {
    pub id: &'static str,
    pub label: &'static str,
    pub short: &'static str,
    pub description: &'static str,
    /// Shown under the description when the skill does work on every turn.
    pub warn: Option<&'static str>,
}

/// The menu order, and the order of every sent and stored list.
pub(crate) const CATALOG: [Skill; 3] = [
    Skill {
        id: "watchtower",
        label: "Watchtower",
        short: "Watch",
        description: "Plan and track watchtower tasks",
        warn: None,
    },
    Skill {
        id: "chase-goal-claude",
        label: "Chase goal",
        short: "Goal",
        description: "Plan and run a goal with subagents",
        warn: Some("Starts goal work on every turn"),
    },
    Skill {
        id: "sequential-thinking",
        label: "Sequential thinking",
        short: "Seq",
        description: "Step-by-step reasoning with revisions",
        warn: None,
    },
];

fn is_known(id: &str) -> bool {
    CATALOG.iter().any(|skill| skill.id == id)
}

/// Pill text: the pinned skills' short names in catalog order joined by a spaced middle dot,
/// or `Skills` when none are pinned (`pillLabel`).
pub(crate) fn pill_label(skills: &[String]) -> String {
    let pinned: Vec<&str> = CATALOG
        .iter()
        .filter(|skill| skills.iter().any(|id| id == skill.id))
        .map(|skill| skill.short)
        .collect();
    if pinned.is_empty() {
        "Skills".into()
    } else {
        pinned.join(" \u{b7} ")
    }
}

/// Catalog order without duplicates and without unknown ids, so equal selections compare equal.
pub(crate) fn normalize(skills: &[String]) -> Vec<String> {
    CATALOG
        .iter()
        .filter(|skill| skills.iter().any(|id| id == skill.id))
        .map(|skill| skill.id.to_string())
        .collect()
}

/// `skillsSchema` over a JSON array: an id outside the catalog is an error.
fn strict_skills(value: &Value) -> Result<Vec<String>> {
    let items = value.as_array().context("skills must be an array")?;
    let mut skills = Vec::with_capacity(items.len());
    for item in items {
        let id = item.as_str().context("a skill id must be a string")?;
        if !is_known(id) {
            bail!("unknown skill id: {id}");
        }
        skills.push(id.to_string());
    }
    Ok(normalize(&skills))
}

/// The answer of `skill-pins.read` and `skill-pins.write`: `{skills}`, validated and ordered.
pub(crate) fn parse_answer(answer: &Value) -> Result<Vec<String>> {
    strict_skills(answer.get("skills").context("the answer has no skills")?)
}

/// `savedSkillsSchema`: a saved list drops the ids of an older catalog instead of failing. A
/// value that is not an array of strings counts as empty.
pub(crate) fn saved_skills(value: Option<&Value>) -> Vec<String> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    let strings: Option<Vec<String>> = items
        .iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect();
    strings.map_or_else(Vec::new, |skills| normalize(&skills))
}

/// `draftSkillsSchema` over the draft value of this plugin: `{skills}` with known ids, or
/// `None` when the draft is untouched or the value does not match.
pub(crate) fn draft_value(value: Option<&Value>) -> Option<Vec<String>> {
    strict_skills(value?.get("skills")?).ok()
}

/// The value a draft save stores under the plugin id.
pub(crate) fn draft_payload(skills: &[String]) -> Value {
    json!({ "skills": skills })
}

/// The pins a draft composer shows: its own value, else the settings defaults, else unknown.
pub(crate) fn draft_skills(
    value: Option<&Value>,
    defaults: Option<Vec<String>>,
) -> Option<Vec<String>> {
    draft_value(value).or(defaults)
}

/// The list after a toggle: a pinned skill leaves, an unpinned one joins in catalog order.
pub(crate) fn toggle(current: &[String], id: &str) -> Vec<String> {
    let mut next: Vec<String> = current.to_vec();
    if next.iter().any(|pinned| pinned == id) {
        next.retain(|pinned| pinned != id);
    } else {
        next.push(id.to_string());
    }
    normalize(&next)
}

/// The `defaults` after one settings switch.
pub(crate) fn set_default(defaults: &[String], id: &str, on: bool) -> Vec<String> {
    let mut next: Vec<String> = defaults.to_vec();
    next.retain(|pinned| pinned != id);
    if on {
        next.push(id.to_string());
    }
    normalize(&next)
}

pub(crate) fn read_input(agent_id: &str) -> Value {
    json!({ "agentId": agent_id })
}

pub(crate) fn write_input(agent_id: &str, skills: &[String]) -> Value {
    json!({ "agentId": agent_id, "skills": skills })
}

/// The hint of a settings switch: the description, or `<description>. <warn>.`.
pub(crate) fn skill_hint(skill: &Skill) -> String {
    match skill.warn {
        Some(warn) => format!("{}. {warn}.", skill.description),
        None => skill.description.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CATALOG, READ, WRITE, draft_skills, draft_value, normalize, parse_answer, pill_label,
        read_input, saved_skills, set_default, skill_hint, toggle, write_input,
    };
    use serde_json::json;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn catalog_holds_the_three_pinned_skills_in_menu_order() {
        let order: Vec<&str> = CATALOG.iter().map(|skill| skill.id).collect();
        assert_eq!(
            order,
            ["watchtower", "chase-goal-claude", "sequential-thinking"]
        );
        let shorts: Vec<&str> = CATALOG.iter().map(|skill| skill.short).collect();
        assert_eq!(shorts, ["Watch", "Goal", "Seq"]);
        assert_eq!(
            CATALOG.iter().filter(|skill| skill.warn.is_some()).count(),
            1
        );
    }

    #[test]
    fn read_answer_sorted_and_deduped() {
        let answer = json!({ "skills": ["sequential-thinking", "watchtower", "watchtower"] });
        assert_eq!(
            parse_answer(&answer).unwrap(),
            ids(&["watchtower", "sequential-thinking"])
        );
    }

    #[test]
    fn pill_label_none_and_two() {
        assert_eq!(pill_label(&[]), "Skills");
        assert_eq!(pill_label(&ids(&["sequential-thinking"])), "Seq");
        assert_eq!(
            pill_label(&ids(&["sequential-thinking", "watchtower"])),
            "Watch \u{b7} Seq"
        );
    }

    #[test]
    fn unknown_rpc_id_is_error() {
        assert!(parse_answer(&json!({ "skills": ["writing-plans"] })).is_err());
        assert!(parse_answer(&json!({ "skills": ["watchtower", 3] })).is_err());
        assert!(parse_answer(&json!({})).is_err());
    }

    #[test]
    fn rpc_wire_names_are_lowercase_with_dots_and_hyphens() {
        for name in [READ, WRITE] {
            let (domain, operation) = name.split_once('.').unwrap();
            assert_eq!(domain, "skill-pins");
            assert!(
                operation
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '-')
            );
        }
    }

    #[test]
    fn wire_inputs_carry_the_agent_and_the_ordered_list() {
        assert_eq!(read_input("a1"), json!({ "agentId": "a1" }));
        assert_eq!(
            write_input("a1", &ids(&["watchtower"])),
            json!({ "agentId": "a1", "skills": ["watchtower"] })
        );
    }

    #[test]
    fn unknown_default_dropped() {
        let saved = json!(["old-skill", "sequential-thinking", "watchtower"]);
        assert_eq!(
            saved_skills(Some(&saved)),
            ids(&["watchtower", "sequential-thinking"])
        );
        assert!(saved_skills(Some(&json!("watchtower"))).is_empty());
        assert!(saved_skills(Some(&json!(["watchtower", 1]))).is_empty());
        assert!(saved_skills(None).is_empty());
    }

    #[test]
    fn draft_value_then_defaults_then_unknown() {
        let defaults = Some(ids(&["sequential-thinking"]));
        let touched = json!({ "skills": ["watchtower"] });
        let shown = draft_skills(Some(&touched), defaults.clone()).unwrap();
        assert_eq!(pill_label(&shown), "Watch");
        let shown = draft_skills(None, defaults.clone()).unwrap();
        assert_eq!(pill_label(&shown), "Seq");
        assert_eq!(draft_skills(None, None), None);
        // A value with an unknown id does not match the schema, so the defaults show.
        let stale = json!({ "skills": ["old-skill"] });
        assert_eq!(draft_skills(Some(&stale), defaults.clone()), defaults);
        // An empty touched draft is a value: it does not fall back to the defaults.
        let cleared = json!({ "skills": [] });
        assert_eq!(draft_value(Some(&cleared)), Some(Vec::new()));
    }

    #[test]
    fn toggle_keeps_catalog_order() {
        let pinned = ids(&["watchtower", "sequential-thinking"]);
        assert_eq!(toggle(&pinned, "watchtower"), ids(&["sequential-thinking"]));
        assert_eq!(
            toggle(&pinned, "chase-goal-claude"),
            ids(&["watchtower", "chase-goal-claude", "sequential-thinking"])
        );
        assert_eq!(
            toggle(&ids(&["sequential-thinking"]), "watchtower"),
            ids(&["watchtower", "sequential-thinking"])
        );
    }

    #[test]
    fn a_settings_switch_sets_or_clears_one_default() {
        let defaults = ids(&["sequential-thinking"]);
        assert_eq!(
            set_default(&defaults, "watchtower", true),
            ids(&["watchtower", "sequential-thinking"])
        );
        assert!(set_default(&defaults, "sequential-thinking", false).is_empty());
        assert_eq!(
            set_default(&defaults, "sequential-thinking", true),
            defaults
        );
    }

    #[test]
    fn normalize_orders_dedupes_and_drops_unknown() {
        assert_eq!(
            normalize(&ids(&[
                "x",
                "sequential-thinking",
                "watchtower",
                "watchtower"
            ])),
            ids(&["watchtower", "sequential-thinking"])
        );
    }

    #[test]
    fn the_hint_adds_the_warning_after_the_description() {
        assert_eq!(skill_hint(&CATALOG[0]), "Plan and track watchtower tasks");
        assert_eq!(
            skill_hint(&CATALOG[1]),
            "Plan and run a goal with subagents. Starts goal work on every turn."
        );
    }
}
