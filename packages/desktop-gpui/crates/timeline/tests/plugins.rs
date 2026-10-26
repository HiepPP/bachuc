//! The plugin rules of the timeline: transformer projection, renderer lookup, the step rule,
//! the selection payload, the reveal match, and the cost of a flush (ADR-0039).

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

use bachuc_plugin_host::{
    ContributionIndex, Contributions, PluginTimelineItem, SelectionResultAction, SourcePlacement,
    SourcePosition, TimelinePhase, TimelineRenderInput, TimelineRenderer, TimelineTransformInput,
    TimelineTransformResult, TimelineTransformer,
};
use bachuc_timeline::grouping::{Row, TurnGrouper, group_turns};
use bachuc_timeline::plugins::{
    Part, ProjectInput, Projector, RevealPlan, StepPart, TOOLBAR_GAP, assistant_selection,
    call_listed, card_origin, card_size, daemon_item, item_id, phase_of, renderer_for_item,
    reveal_plan, selection_anchor, shown_parts, shown_text, step_parts, toolbar_origin,
    transform_entry, unique_match,
};
use gpui_kit::{AnyView, App, Task, Window, bounds, point, px, size};
use serde_json::{Value, json};

type Transform = dyn Fn(&TimelineTransformInput<'_>) -> Option<TimelineTransformResult>;

fn transformer(
    item_type: &'static str,
    transform: impl Fn(&TimelineTransformInput<'_>) -> Option<TimelineTransformResult> + 'static,
) -> TimelineTransformer {
    let transform: Rc<Transform> = Rc::new(transform);
    TimelineTransformer {
        id: "t",
        item_type,
        transform,
    }
}

fn renderer(kind: &'static str, version: u32) -> TimelineRenderer {
    TimelineRenderer {
        kind,
        version,
        view: Rc::new(
            |_: TimelineRenderInput, _: &mut Window, _: &mut App| -> AnyView {
                unreachable!("not drawn in a pure test")
            },
        ),
    }
}

fn index(plugins: &[(&'static str, &Contributions)]) -> ContributionIndex {
    ContributionIndex::build(plugins.iter().map(|(id, c)| (*id, *c)))
}

fn plugin_item(kind: &str, version: u32, id: Option<&str>) -> PluginTimelineItem {
    PluginTimelineItem {
        id: id.map(str::to_string),
        kind: kind.to_string(),
        version,
        data: json!({ "kind": kind }),
    }
}

fn result(
    items: Vec<PluginTimelineItem>,
    source: Option<SourcePlacement>,
) -> TimelineTransformResult {
    TimelineTransformResult { items, source }
}

fn entry(seq: u64, item: Value) -> Value {
    json!({
        "seqStart": seq, "seqEnd": seq, "timestamp": "2026-01-01T00:00:00.000Z", "item": item
    })
}

fn call(seq: u64, status: &str) -> Value {
    entry(
        seq,
        json!({
            "type": "tool_call", "callId": format!("c{seq}"), "name": "edit", "status": status,
            "error": null, "detail": { "type": "edit", "filePath": "/a.rs" }
        }),
    )
}

fn assistant(seq: u64, text: &str) -> Value {
    entry(seq, json!({ "type": "assistant_message", "text": text }))
}

fn user(seq: u64, text: &str) -> Value {
    entry(seq, json!({ "type": "user_message", "text": text }))
}

fn no_labels() -> BTreeMap<String, String> {
    BTreeMap::new()
}

fn input<'a>(
    index: &'a ContributionIndex,
    revision: u64,
    labels: &'a BTreeMap<String, String>,
    turn_active: bool,
) -> ProjectInput<'a> {
    ProjectInput {
        index: Some(index),
        revision,
        agent_id: "agent",
        labels,
        turn_active,
    }
}

#[test]
fn first_some_wins_in_index_order() {
    let mut first = Contributions::default();
    first.add_timeline_transformer(transformer("tool_call", |_| None));
    let mut second = Contributions::default();
    second.add_timeline_transformer(transformer("tool_call", |_| {
        Some(result(vec![plugin_item("edit-diff", 1, None)], None))
    }));
    let mut third = Contributions::default();
    third.add_timeline_transformer(transformer("tool_call", |_| {
        Some(result(vec![plugin_item("other", 1, None)], None))
    }));
    let index = index(&[("one", &first), ("two", &second), ("three", &third)]);
    let projection = transform_entry(
        index.transformers_for("tool_call"),
        "agent",
        &no_labels(),
        &call(5, "completed"),
        TimelinePhase::Complete,
    )
    .expect("the second transformer's result");
    assert_eq!(projection.plugin_id, "two");
    assert_eq!(projection.items[0].item.kind, "edit-diff");
}

#[test]
fn a_bad_kind_or_version_counts_as_none() {
    for bad in [
        plugin_item("Bad Kind", 1, None),
        plugin_item("diff", 0, None),
        plugin_item("diff", 1, Some("")),
    ] {
        let mut only = Contributions::default();
        let item = bad.clone();
        only.add_timeline_transformer(transformer("tool_call", move |_| {
            Some(result(vec![item.clone()], None))
        }));
        let alone = index(&[("p", &only)]);
        assert_eq!(
            transform_entry(
                alone.transformers_for("tool_call"),
                "agent",
                &no_labels(),
                &call(1, "completed"),
                TimelinePhase::Complete,
            ),
            None,
            "{bad:?} is dropped and the entry draws as without plugins"
        );
        // The next transformer still gets its turn.
        let mut next = Contributions::default();
        next.add_timeline_transformer(transformer("tool_call", |_| {
            Some(result(vec![plugin_item("good-kind-2", 1, None)], None))
        }));
        let both = index(&[("p", &only), ("q", &next)]);
        let projection = transform_entry(
            both.transformers_for("tool_call"),
            "agent",
            &no_labels(),
            &call(1, "completed"),
            TimelinePhase::Complete,
        )
        .unwrap();
        assert_eq!(projection.plugin_id, "q");
    }
}

#[test]
fn items_without_a_source_replace_the_entry() {
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("tool_call", |_| {
        Some(result(vec![plugin_item("edit-diff", 1, Some("d1"))], None))
    }));
    let index = index(&[("edits", &plugin)]);
    let projection = transform_entry(
        index.transformers_for("tool_call"),
        "agent",
        &no_labels(),
        &call(9, "completed"),
        TimelinePhase::Complete,
    )
    .unwrap();
    assert!(!projection.is_hidden());
    assert_eq!(projection.parts(), vec![Part::Item(&projection.items[0])]);
    // In a step, the plugin item takes the call's place, and the call leaves the call list.
    assert_eq!(
        step_parts(Some(&projection), true),
        vec![StepPart::Item(&projection.items[0])]
    );
    assert!(!call_listed(Some(&projection)));
}

#[test]
fn a_first_source_with_text_shows_the_new_text_then_the_items() {
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("assistant_message", |_| {
        Some(result(
            vec![plugin_item("card", 1, None)],
            Some(SourcePlacement {
                position: SourcePosition::First,
                text: Some("rewritten".into()),
            }),
        ))
    }));
    let index = index(&[("p", &plugin)]);
    let message = assistant(3, "original");
    let projection = transform_entry(
        index.transformers_for("assistant_message"),
        "agent",
        &no_labels(),
        &message,
        TimelinePhase::Complete,
    )
    .unwrap();
    assert_eq!(shown_text(&message, Some(&projection)), "rewritten");
    assert_eq!(shown_text(&message, None), "original");
    assert_eq!(
        projection.parts(),
        vec![Part::Source, Part::Item(&projection.items[0])]
    );
}

#[test]
fn a_last_source_keeps_the_call_unchanged_after_the_items() {
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("tool_call", |_| {
        Some(result(
            vec![plugin_item("note", 1, None)],
            Some(SourcePlacement {
                position: SourcePosition::Last,
                text: Some("ignored for calls".into()),
            }),
        ))
    }));
    let index = index(&[("p", &plugin)]);
    let edit = call(4, "completed");
    let projection = transform_entry(
        index.transformers_for("tool_call"),
        "agent",
        &no_labels(),
        &edit,
        TimelinePhase::Complete,
    )
    .unwrap();
    assert_eq!(shown_text(&edit, Some(&projection)), "");
    assert_eq!(
        projection.parts(),
        vec![Part::Item(&projection.items[0]), Part::Source]
    );
    // The kept edit keeps its tool call row, after the item; a kept call stays in the call list.
    assert_eq!(
        step_parts(Some(&projection), true),
        vec![StepPart::Item(&projection.items[0]), StepPart::ToolCall]
    );
    assert!(call_listed(Some(&projection)));
}

/// An item with no renderer draws nothing; the kept source still draws, and an entry with no
/// part left shows nothing.
#[test]
fn an_item_with_no_renderer_is_hidden() {
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("assistant_message", |_| {
        Some(result(
            vec![
                plugin_item("html-reply", 1, None),
                plugin_item("card", 1, None),
            ],
            Some(SourcePlacement {
                position: SourcePosition::First,
                text: None,
            }),
        ))
    }));
    plugin.add_timeline_transformer(transformer("user_message", |_| {
        Some(result(vec![plugin_item("html-reply", 1, None)], None))
    }));
    plugin.add_timeline_renderer(renderer("card", 1));
    let index = index(&[("p", &plugin)]);
    let has_renderer = |plugin_id: &'static str| {
        let index = &index;
        move |item: &bachuc_timeline::plugins::ProjectedItem| {
            renderer_for_item(index, plugin_id, &item.item.kind, item.item.version).is_some()
        }
    };
    let project = |item_type: &str, entry: &Value| {
        transform_entry(
            index.transformers_for(item_type),
            "agent",
            &no_labels(),
            entry,
            TimelinePhase::Complete,
        )
        .unwrap()
    };

    // A renderer: the item shows. None: it is hidden, and the kept source stays.
    let kept = project("assistant_message", &assistant(1, "hi"));
    assert_eq!(
        shown_parts(&kept, has_renderer("p")),
        vec![Part::Source, Part::Item(&kept.items[1])]
    );
    assert_eq!(shown_parts(&kept, |_| false), vec![Part::Source]);

    // Items that replace the entry, none with a renderer: nothing is left to draw.
    let replaced = project("user_message", &user(2, "hi"));
    assert!(shown_parts(&replaced, has_renderer("p")).is_empty());
    // A renderer of another plugin does not show it.
    assert!(shown_parts(&kept, has_renderer("someone-else")).len() == 1);
}

#[test]
fn no_items_and_no_source_hide_the_entry_and_keep_grouping() {
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("tool_call", |_| Some(result(vec![], None))));
    let index = index(&[("p", &plugin)]);
    let entries = vec![user(1, "go"), call(2, "completed"), assistant(3, "done")];
    let before = group_turns(&entries, false);
    let mut grouper = TurnGrouper::default();
    grouper.sync(&entries, false);
    let mut projector = Projector::default();
    let labels = no_labels();
    projector.sync(
        &input(&index, 1, &labels, false),
        &entries,
        grouper.changed(),
        false,
    );
    assert!(projector.is_hidden(1));
    assert!(!projector.is_hidden(0));
    assert_eq!(step_parts(projector.get(1).map(|p| &**p), true), vec![]);
    assert!(!call_listed(projector.get(1).map(|p| &**p)));
    assert_eq!(
        group_turns(&entries, false),
        before,
        "grouping reads the source entries"
    );
}

#[test]
fn item_ids_take_the_plugin_id() {
    assert_eq!(item_id("edits", Some("d1"), 7, 0), "edits/d1");
    assert_eq!(item_id("edits", None, 7, 2), "edits/7/2");
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("tool_call", |_| {
        Some(result(
            vec![plugin_item("a", 1, Some("x")), plugin_item("b", 1, None)],
            None,
        ))
    }));
    let index = index(&[("edits", &plugin)]);
    let projection = transform_entry(
        index.transformers_for("tool_call"),
        "agent",
        &no_labels(),
        &call(12, "completed"),
        TimelinePhase::Complete,
    )
    .unwrap();
    let ids: Vec<&str> = projection
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(ids, ["edits/x", "edits/12/1"]);
}

#[test]
fn phases_follow_running_calls_and_the_streaming_message() {
    assert_eq!(
        phase_of(&call(1, "running"), false),
        TimelinePhase::Streaming
    );
    assert_eq!(
        phase_of(&call(1, "completed"), false),
        TimelinePhase::Complete
    );
    assert_eq!(phase_of(&assistant(1, "x"), true), TimelinePhase::Streaming);
    assert_eq!(phase_of(&assistant(1, "x"), false), TimelinePhase::Complete);

    // The projector gives the streaming phase to the last assistant message or thought only
    // while the turn runs.
    let seen: Rc<std::cell::RefCell<Vec<(String, TimelinePhase)>>> = Rc::default();
    let record = seen.clone();
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("assistant_message", move |input| {
        record.borrow_mut().push((
            input.item["text"].as_str().unwrap().to_string(),
            input.phase,
        ));
        None
    }));
    let index = index(&[("p", &plugin)]);
    let labels = no_labels();
    let mut entries = vec![user(1, "go"), assistant(2, "a"), assistant(3, "b")];
    let mut grouper = TurnGrouper::default();
    let mut projector = Projector::default();
    grouper.sync(&entries, true);
    projector.sync(
        &input(&index, 1, &labels, true),
        &entries,
        grouper.changed(),
        false,
    );
    assert_eq!(
        *seen.borrow(),
        [
            ("a".to_string(), TimelinePhase::Complete),
            ("b".to_string(), TimelinePhase::Streaming)
        ]
    );
    // A new message streams; the old one completes, though its entry did not change.
    seen.borrow_mut().clear();
    entries.push(assistant(4, "c"));
    grouper.sync(&entries, true);
    projector.sync(
        &input(&index, 1, &labels, true),
        &entries,
        grouper.changed(),
        false,
    );
    assert_eq!(
        *seen.borrow(),
        [
            ("b".to_string(), TimelinePhase::Complete),
            ("c".to_string(), TimelinePhase::Streaming)
        ]
    );
    // The turn ends: the last message completes.
    seen.borrow_mut().clear();
    grouper.sync(&entries, false);
    projector.sync(
        &input(&index, 1, &labels, false),
        &entries,
        grouper.changed(),
        false,
    );
    assert_eq!(*seen.borrow(), [("c".to_string(), TimelinePhase::Complete)]);
    // A new prompt starts the next turn before its reply: the old reply stays complete.
    seen.borrow_mut().clear();
    entries.push(user(5, "ok"));
    grouper.sync(&entries, true);
    projector.sync(
        &input(&index, 1, &labels, true),
        &entries,
        grouper.changed(),
        false,
    );
    assert!(seen.borrow().is_empty());
    entries.push(assistant(6, "d"));
    grouper.sync(&entries, true);
    projector.sync(
        &input(&index, 1, &labels, true),
        &entries,
        grouper.changed(),
        false,
    );
    assert_eq!(
        *seen.borrow(),
        [("d".to_string(), TimelinePhase::Streaming)]
    );
}

#[test]
fn daemon_items_use_only_their_own_plugins_renderer() {
    let mut owner = Contributions::default();
    owner.add_timeline_renderer(renderer("chart", 1));
    let index = index(&[("charts", &owner)]);
    let item = json!({
        "type": "plugin", "id": "i1", "pluginId": "charts", "kind": "chart", "version": 1,
        "data": { "points": [1, 2] }
    });
    let (plugin_id, row_id, parsed) = daemon_item(&item).unwrap();
    assert_eq!(
        (plugin_id.as_str(), row_id.as_str()),
        ("charts", "charts/i1")
    );
    assert_eq!(parsed.data, json!({ "points": [1, 2] }));
    assert!(renderer_for_item(&index, &plugin_id, &parsed.kind, parsed.version).is_some());
    // A renderer of the same kind from another plugin does not draw it.
    assert!(renderer_for_item(&index, "someone-else", "chart", 1).is_none());
    assert!(renderer_for_item(&index, "charts", "chart", 2).is_none());
    assert_eq!(daemon_item(&json!({ "type": "tool_call" })), None);
}

/// Without the edit-diffs plugin, an edit call in a step draws as its tool call row, always;
/// another call stays a count.
#[test]
fn an_edit_call_without_a_projection_draws_as_a_tool_call_in_its_step() {
    assert_eq!(step_parts(None, true), vec![StepPart::ToolCall]);
    assert_eq!(step_parts(None, false), vec![]);
    assert!(call_listed(None));
    let entries = vec![user(1, "go"), call(2, "completed"), assistant(3, "done")];
    let rows = group_turns(&entries, false);
    let step = rows
        .iter()
        .find_map(|row| match row {
            Row::Turn(group) => group.steps.first(),
            Row::Item(_) => None,
        })
        .expect("a turn step");
    assert_eq!(step.edits, vec![1]);
    let parts: Vec<StepPart> = step
        .items
        .iter()
        .flat_map(|&index| step_parts(None, step.edits.contains(&index)))
        .collect();
    assert_eq!(parts, vec![StepPart::ToolCall]);
}

/// A label that turns the transformer off projects every entry again once, then only the
/// changed entries; a revision change runs the transformer once per matching entry.
#[test]
fn labels_and_revision_changes_project_every_entry_once() {
    let runs = Rc::new(Cell::new(0));
    let counter = runs.clone();
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("tool_call", move |input| {
        counter.set(counter.get() + 1);
        (input.labels.get("diffs").map(String::as_str) != Some("off"))
            .then(|| result(vec![plugin_item("edit-diff", 1, None)], None))
    }));
    let index = index(&[("edits", &plugin)]);
    let mut entries: Vec<Value> = (1..=10).map(|seq| call(seq, "completed")).collect();
    entries.push(assistant(11, "done"));
    let mut grouper = TurnGrouper::default();
    let mut projector = Projector::default();
    let on = no_labels();
    grouper.sync(&entries, false);
    let outcome = projector.sync(
        &input(&index, 1, &on, false),
        &entries,
        grouper.changed(),
        false,
    );
    assert!(outcome.full);
    assert_eq!(runs.get(), 10, "once per tool call");
    assert!(projector.get(0).is_some());

    let off = BTreeMap::from([("diffs".to_string(), "off".to_string())]);
    grouper.sync(&entries, false);
    let outcome = projector.sync(
        &input(&index, 1, &off, false),
        &entries,
        grouper.changed(),
        false,
    );
    assert!(outcome.full);
    assert_eq!(runs.get(), 20);
    assert_eq!(outcome.changed, (0..10).collect::<Vec<_>>());
    assert!(projector.get(0).is_none());

    entries.push(call(12, "completed"));
    grouper.sync(&entries, false);
    let outcome = projector.sync(
        &input(&index, 1, &off, false),
        &entries,
        grouper.changed(),
        false,
    );
    assert!(!outcome.full);
    assert_eq!(runs.get(), 21, "only the new entry");

    grouper.sync(&entries, false);
    projector.sync(
        &input(&index, 2, &off, false),
        &entries,
        grouper.changed(),
        false,
    );
    assert_eq!(runs.get(), 32, "a revision runs every tool call again");
}

#[test]
fn the_selection_payload_prefers_the_markdown_source() {
    let source = "Use **foo** here";
    let with_range = assistant_selection(
        "agent",
        Some("ws".into()),
        "m1".into(),
        source,
        Some(4..11),
        "foo".into(),
    )
    .unwrap();
    assert_eq!(with_range.text, "**foo**");
    assert_eq!(with_range.message_id.as_deref(), Some("m1"));
    assert_eq!(with_range.workspace_id.as_deref(), Some("ws"));
    assert_eq!(with_range.agent_id, "agent");
    let without =
        assistant_selection("agent", None, "seq:4".into(), source, None, "foo".into()).unwrap();
    assert_eq!(without.text, "foo");
    assert_eq!(without.message_id.as_deref(), Some("seq:4"));
    assert_eq!(
        assistant_selection("agent", None, "m1".into(), source, None, String::new()),
        None
    );
}

#[test]
fn the_toolbar_anchors_to_the_selected_lines_not_the_whole_message() {
    let view = bounds(point(px(40.), px(100.)), size(px(1000.), px(500.)));
    // A selection from the middle of one line to the start of the next.
    let start = bounds(point(px(700.), px(540.)), size(px(2.), px(24.)));
    let end = bounds(point(px(40.), px(564.)), size(px(2.), px(24.)));
    assert_eq!(
        selection_anchor(view, Some((start, end))),
        bounds(point(px(40.), px(540.)), size(px(662.), px(48.)))
    );
    // Nothing painted yet: the whole view.
    assert_eq!(selection_anchor(view, None), view);
}

#[test]
fn the_toolbar_sits_above_the_text_inside_the_timeline() {
    let area = bounds(point(px(100.), px(50.)), size(px(400.), px(600.)));
    let toolbar = size(px(120.), px(30.));
    let text = bounds(point(px(150.), px(300.)), size(px(200.), px(80.)));
    // Centered on 250 (150 relative), 8 above the text's top.
    assert_eq!(TOOLBAR_GAP, 8.);
    assert_eq!(
        toolbar_origin(text, toolbar, area, px(TOOLBAR_GAP)),
        Some(point(px(90.), px(212.)))
    );
    // Near the left edge it stays inside.
    let left = bounds(point(px(100.), px(300.)), size(px(20.), px(20.)));
    assert_eq!(
        toolbar_origin(left, toolbar, area, px(TOOLBAR_GAP))
            .unwrap()
            .x,
        px(0.)
    );
    // Text above the top of the timeline: the toolbar holds at the top.
    let above = bounds(point(px(150.), px(20.)), size(px(200.), px(200.)));
    assert_eq!(
        toolbar_origin(above, toolbar, area, px(TOOLBAR_GAP))
            .unwrap()
            .y,
        px(0.)
    );
    // Text scrolled out: no toolbar.
    let out = bounds(point(px(150.), px(700.)), size(px(200.), px(20.)));
    assert_eq!(toolbar_origin(out, toolbar, area, px(TOOLBAR_GAP)), None);
}

#[test]
fn the_result_card_fits_its_content_up_to_420_by_320_times_the_zoom() {
    // Small content keeps its size.
    assert_eq!(
        card_size(size(px(200.), px(100.)), 1.),
        size(px(200.), px(100.))
    );
    // Long content is capped, and the cap grows with the zoom.
    assert_eq!(
        card_size(size(px(900.), px(900.)), 1.),
        size(px(420.), px(320.))
    );
    assert_eq!(
        card_size(size(px(900.), px(900.)), 1.25),
        size(px(525.), px(400.))
    );
    // Each side is capped alone.
    assert_eq!(
        card_size(size(px(300.), px(900.)), 1.),
        size(px(300.), px(320.))
    );
}

#[test]
fn the_result_card_stays_inside_the_timeline() {
    let area = size(px(800.), px(600.));
    let card = size(px(420.), px(320.));
    // Room enough: the toolbar's origin holds.
    assert_eq!(
        card_origin(point(px(100.), px(50.)), card, area),
        point(px(100.), px(50.))
    );
    // Past the right and bottom edges it moves back in.
    assert_eq!(
        card_origin(point(px(700.), px(500.)), card, area),
        point(px(380.), px(280.))
    );
    // A card larger than the timeline sits at the corner.
    assert_eq!(
        card_origin(point(px(30.), px(30.)), card, size(px(300.), px(200.))),
        point(px(0.), px(0.))
    );
}

#[test]
fn the_index_lists_result_actions_after_registration_order() {
    let mut contributions = Contributions::default();
    for id in ["first", "second"] {
        contributions.add_selection_result_action(SelectionResultAction {
            id,
            title: id.into(),
            loading: "Loading...".into(),
            run: Rc::new(|_, _| Task::ready(Ok(String::new()))),
        });
    }
    let index = ContributionIndex::build([("demo", &contributions)]);
    let ids: Vec<_> = index
        .selection_result_actions()
        .iter()
        .map(|action| (action.plugin_id, action.value.id))
        .collect();
    assert_eq!(ids, [("demo", "first"), ("demo", "second")]);
    assert!(index.selection_actions().is_empty());
}

#[test]
fn a_passage_highlights_only_a_unique_match() {
    assert_eq!(unique_match(&["one two", "three"], "two"), Some((0, 4..7)));
    assert_eq!(
        unique_match(&["one", "two three"], "three"),
        Some((1, 4..9))
    );
    assert_eq!(
        unique_match(&["two", "two"], "two"),
        None,
        "found twice: scroll only"
    );
    assert_eq!(unique_match(&["two two"], "two"), None);
    assert_eq!(unique_match(&["one"], "four"), None);
    assert_eq!(unique_match(&["one"], ""), None);
    assert_eq!(reveal_plan(true, true), RevealPlan::Reveal);
    assert_eq!(reveal_plan(true, false), RevealPlan::NotLoaded);
    assert_eq!(reveal_plan(false, false), RevealPlan::Wait);
}

/// ADR-0039: appending one entry to a 20,000-entry timeline runs the one `tool_call`
/// transformer once, and the projection step stays under 2 ms in a debug build.
#[test]
fn appending_to_20000_entries_transforms_once() {
    let runs = Rc::new(Cell::new(0));
    let counter = runs.clone();
    let mut plugin = Contributions::default();
    plugin.add_timeline_transformer(transformer("tool_call", move |_| {
        counter.set(counter.get() + 1);
        Some(result(vec![plugin_item("edit-diff", 1, None)], None))
    }));
    let index = index(&[("edits", &plugin)]);
    let labels = no_labels();
    let mut entries: Vec<Value> = vec![user(0, "go")];
    entries.extend((1..20_000).map(|seq| call(seq, "completed")));
    let mut grouper = TurnGrouper::default();
    let mut projector = Projector::default();
    grouper.sync(&entries, true);
    projector.sync(
        &input(&index, 1, &labels, true),
        &entries,
        grouper.changed(),
        false,
    );
    assert_eq!(runs.get(), 19_999);

    entries.push(call(20_000, "running"));
    grouper.sync(&entries, true);
    let changed = grouper.changed().to_vec();
    let started = Instant::now();
    let outcome = projector.sync(&input(&index, 1, &labels, true), &entries, &changed, false);
    let took = started.elapsed();
    assert_eq!(runs.get(), 20_000, "one run for the appended entry");
    assert_eq!(outcome.changed, vec![20_000]);
    eprintln!("projection step after one append: {took:?}");
    assert!(took.as_secs_f64() * 1000. < 2., "took {took:?}");

    grouper.sync(&entries, true);
    projector.sync(
        &input(&index, 2, &labels, true),
        &entries,
        grouper.changed(),
        false,
    );
    assert_eq!(runs.get(), 40_000, "a revision runs once per tool call");
}
