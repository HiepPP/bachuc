//! The `next-prompts` v1 declaration and its relation checks (`shared/next-prompts.ts`).

use crate::contracts::Candidate;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::sync::LazyLock;

static ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]*$").unwrap());
const MAX_BLOCK: usize = 65536;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeclaredPrompt {
    pub id: String,
    pub prompt: String,
    pub why: Option<String>,
    /// Unrelated to the current goal: offered only as a separate new thread.
    pub thread_new: bool,
    /// Display only. Any non-boolean value counts as not suggested.
    pub suggestion: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NextPromptsV1 {
    pub prompts: Vec<DeclaredPrompt>,
    pub goal_done: bool,
    pub exclusive_groups: Vec<Vec<String>>,
    pub allowed_combinations: Vec<Vec<String>>,
}

/// JS string length, which counts UTF-16 units.
pub(crate) fn js_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn id(value: &Value) -> Option<String> {
    let text = value.as_str()?;
    (js_len(text) <= 64 && ID.is_match(text)).then(|| text.to_string())
}

/// `z.string().max(limit).refine(nonempty)`.
fn nonempty(value: &Value, limit: usize) -> Option<String> {
    let text = value.as_str()?;
    (js_len(text) <= limit && !text.trim().is_empty()).then(|| text.to_string())
}

/// `z.array(ids).max(limit)` with `ids` of 2 to 20 ids; an absent key is `[]`.
fn relations(object: &Map<String, Value>, key: &str, limit: usize) -> Option<Vec<Vec<String>>> {
    let Some(value) = object.get(key) else {
        return Some(Vec::new());
    };
    let lists = value.as_array()?;
    if lists.len() > limit {
        return None;
    }
    lists
        .iter()
        .map(|list| {
            let items = list.as_array()?;
            if !(2..=20).contains(&items.len()) {
                return None;
            }
            items.iter().map(id).collect()
        })
        .collect()
}

fn prompt(value: &Value) -> Option<DeclaredPrompt> {
    let object = value.as_object()?;
    let why = match object.get("why") {
        None => None,
        Some(why) => Some(nonempty(why, 2000)?),
    };
    let thread_new = match object.get("thread") {
        None => false,
        Some(thread) => {
            if thread.as_str() != Some("new") {
                return None;
            }
            true
        }
    };
    Some(DeclaredPrompt {
        id: id(object.get("id")?)?,
        prompt: nonempty(object.get("prompt")?, 16000)?,
        why,
        thread_new,
        suggestion: object
            .get("suggestion")
            .map(|value| value.as_bool().unwrap_or(false)),
    })
}

/// The schema without the relation checks. Unknown fields are ignored, not rejected.
fn shape(value: &Value) -> Option<NextPromptsV1> {
    let object = value.as_object()?;
    if object.get("version")?.as_f64() != Some(1.) {
        return None;
    }
    let prompts = object.get("prompts")?.as_array()?;
    if !(1..=20).contains(&prompts.len()) {
        return None;
    }
    let goal_done = match object.get("goal") {
        None => false,
        Some(goal) => {
            if goal.as_str() != Some("done") {
                return None;
            }
            true
        }
    };
    Some(NextPromptsV1 {
        prompts: prompts.iter().map(prompt).collect::<Option<_>>()?,
        goal_done,
        exclusive_groups: relations(object, "exclusiveGroups", 20)?,
        allowed_combinations: relations(object, "allowedCombinations", 64)?,
    })
}

/// The `superRefine` of the declaration schema.
fn relations_valid(value: &NextPromptsV1) -> bool {
    let known: BTreeSet<&str> = value.prompts.iter().map(|p| p.id.as_str()).collect();
    if known.len() != value.prompts.len() {
        return false;
    }
    let separate: BTreeSet<&str> = value
        .prompts
        .iter()
        .filter(|p| p.thread_new)
        .map(|p| p.id.as_str())
        .collect();
    for list in value
        .exclusive_groups
        .iter()
        .chain(&value.allowed_combinations)
    {
        let unique: BTreeSet<&str> = list.iter().map(String::as_str).collect();
        if unique.len() != list.len()
            || list
                .iter()
                .any(|key| !known.contains(key.as_str()) || separate.contains(key.as_str()))
        {
            return false;
        }
    }
    // Disjoint groups map directly to radio groups.
    let mut grouped = BTreeSet::new();
    for key in value.exclusive_groups.iter().flatten() {
        if !grouped.insert(key.as_str()) {
            return false;
        }
    }
    let mut combinations = BTreeSet::new();
    for combination in &value.allowed_combinations {
        let mut sorted = combination.clone();
        sorted.sort();
        if !combinations.insert(sorted) {
            return false;
        }
        if value.exclusive_groups.iter().any(|group| {
            group
                .iter()
                .filter(|item| combination.contains(item))
                .count()
                > 1
        }) {
            return false;
        }
    }
    true
}

/// Agents often omit `id` when nothing refers to it; default it to the 1-based position.
fn default_ids(value: &mut Value) {
    let Some(prompts) = value.get_mut("prompts").and_then(Value::as_array_mut) else {
        return;
    };
    for (index, prompt) in prompts.iter_mut().enumerate() {
        if let Some(object) = prompt.as_object_mut() {
            object
                .entry("id")
                .or_insert_with(|| Value::String(format!("p{}", index + 1)));
        }
    }
}

pub(crate) fn parse_next_prompts(block: &str) -> Option<NextPromptsV1> {
    if js_len(block) > MAX_BLOCK || block.len() > MAX_BLOCK {
        return None;
    }
    let mut value: Value = serde_json::from_str(block).ok()?;
    default_ids(&mut value);
    shape(&value).filter(relations_valid)
}

/// Whether the candidates may go out as one message: one alone, or a declared combination of
/// one block with no two of an exclusive group.
pub(crate) fn selection_allowed(candidates: &[&Candidate]) -> bool {
    let keys: BTreeSet<&str> = candidates.iter().map(|c| c.key.as_str()).collect();
    if candidates.is_empty() || keys.len() != candidates.len() {
        return false;
    }
    if candidates.len() == 1 {
        return true;
    }
    let Some(first) = &candidates[0].selection else {
        return false;
    };
    if candidates.iter().any(|c| {
        c.selection
            .as_ref()
            .is_none_or(|selection| selection.block_key != first.block_key)
    }) {
        return false;
    }
    let selected: BTreeSet<&str> = candidates
        .iter()
        .filter_map(|c| c.selection.as_ref().map(|s| s.id.as_str()))
        .collect();
    if selected.len() != candidates.len() {
        return false;
    }
    if first.exclusive_groups.iter().any(|group| {
        group
            .iter()
            .filter(|id| selected.contains(id.as_str()))
            .count()
            > 1
    }) {
        return false;
    }
    first.allowed_combinations.iter().any(|group| {
        group.len() == selected.len() && group.iter().all(|id| selected.contains(id.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::selection_allowed;
    use crate::contracts::{Candidate, Selection};

    fn candidate(key: &str, id: &str, block: &str) -> Candidate {
        Candidate {
            key: key.into(),
            selection: Some(Selection {
                id: id.into(),
                block_key: block.into(),
                exclusive_groups: vec![vec!["x".into(), "y".into()]],
                allowed_combinations: vec![vec!["x".into(), "risks".into()]],
            }),
            ..Candidate::default()
        }
    }

    #[test]
    fn selection_allowed_takes_only_declared_combinations() {
        let x = candidate("k1", "x", "b");
        let y = candidate("k2", "y", "b");
        let risks = candidate("k3", "risks", "b");
        let other = candidate("k4", "risks", "other");
        assert!(selection_allowed(&[&x]));
        assert!(selection_allowed(&[&x, &risks]));
        assert!(!selection_allowed(&[&x, &y]), "one exclusive group");
        assert!(!selection_allowed(&[&y, &risks]), "not declared");
        assert!(!selection_allowed(&[&x, &other]), "two blocks");
        assert!(!selection_allowed(&[&x, &x]), "duplicate key");
        assert!(!selection_allowed(&[]));
    }
}
