//! The permission card answers, as `PermissionRequestCard` in
//! `packages/app/src/agent-stream/view.tsx` and `question-form-card-core.ts` build them, and the
//! find and image helpers of the agent pane.

use bachuc_client::SentImage;
use bachuc_timeline::cards::{
    QuestionForm, action_response, actions, dismiss_label, parse_questions, plan_outcome,
    plan_starts_open, plan_text, plan_title, title,
};
use bachuc_timeline::find::{match_ranges, status, step};
use bachuc_timeline::{decode_sent_image, display_size, image_dimensions, read_target};
use serde_json::json;

#[test]
fn a_plan_request_offers_deny_and_implement_and_rejects_with_the_app_payload() {
    let request = json!({
        "id": "p1", "kind": "plan", "name": "ExitPlanMode",
        "input": {"plan": "1. Edit"}, "metadata": {"planText": "## Plan\n1. Edit"}
    });
    assert_eq!(title(&request), "Plan");
    assert_eq!(plan_text(&request).as_deref(), Some("## Plan\n1. Edit"));
    let buttons = actions(&request);
    let labels: Vec<&str> = buttons.iter().map(|a| a.label.as_str()).collect();
    assert_eq!(labels, ["Deny", "Implement"]);
    assert_eq!(
        action_response(&buttons[0]),
        json!({"behavior": "deny", "selectedActionId": "reject", "message": "Denied by user"})
    );
    assert_eq!(
        action_response(&buttons[1]),
        json!({"behavior": "allow", "selectedActionId": "accept"})
    );
}

#[test]
fn a_request_with_its_own_actions_keeps_them() {
    let request = json!({
        "id": "t1", "kind": "tool", "title": "Run npm test",
        "actions": [
            {"id": "allow_always", "label": "Always allow", "behavior": "allow", "variant": "primary"},
            {"id": "no", "label": "No", "behavior": "deny"}
        ]
    });
    assert_eq!(title(&request), "Run npm test");
    let buttons = actions(&request);
    assert!(buttons[0].primary && buttons[0].allow);
    assert_eq!(
        action_response(&buttons[1]),
        json!({"behavior": "deny", "selectedActionId": "no", "message": "Denied by user"})
    );
    assert_eq!(title(&json!({"kind": "tool"})), "Permission Required");
}

#[test]
fn a_plan_tool_call_titles_and_folds_by_its_outcome() {
    let rejected = json!({"status": "completed", "metadata": {"approved": false}});
    assert_eq!(plan_title(plan_outcome(&rejected), "Plan"), "Rejected plan");
    assert!(!plan_starts_open(plan_outcome(&rejected)));
    let pending = json!({"status": "running"});
    assert_eq!(plan_title(plan_outcome(&pending), "Plan"), "Plan");
    assert!(plan_starts_open(plan_outcome(&pending)));
    let canceled = json!({"status": "canceled", "metadata": {"approved": true}});
    assert_eq!(plan_title(plan_outcome(&canceled), "Plan"), "Canceled plan");
}

fn question_input() -> serde_json::Value {
    json!({"questions": [
        {"question": "Which db?", "header": "Database", "multiSelect": false,
         "options": [{"label": "Postgres", "description": "SQL"}, {"label": "Mongo"}]},
        {"question": "Which extras?", "header": "Extras", "multiSelect": true, "allowOther": true,
         "options": [{"label": "Cache"}, {"label": "Queue"}]}
    ]})
}

#[test]
fn a_question_form_steps_through_its_questions_and_submits_the_answers() {
    let input = question_input();
    let questions = parse_questions(&input).unwrap();
    let mut form = QuestionForm::new(questions.len());
    assert!(!form.can_advance(&questions));
    // A single-select pick answers the question and moves on.
    form.toggle(&questions, 0, 1);
    assert_eq!(form.active, 1);
    assert!(!form.can_advance(&questions));
    // Multi select keeps the pick order and adds the custom answer last.
    form.toggle(&questions, 1, 1);
    form.toggle(&questions, 1, 0);
    form.set_other(&questions, 1, "  Search ".into());
    assert!(form.is_last(&questions) && form.can_advance(&questions));
    assert_eq!(
        form.submit_response(&questions, &input),
        json!({"behavior": "allow", "updatedInput": {
            "questions": input["questions"],
            "answers": {"Database": "Mongo", "Extras": "Queue, Cache, Search"}
        }})
    );
    assert_eq!(
        form.dismiss_response(&questions, &input),
        json!({"behavior": "deny", "message": "Dismissed by user"})
    );
    assert_eq!(dismiss_label(&questions), "Dismiss");
}

#[test]
fn a_custom_answer_replaces_a_single_select_pick() {
    let input = json!({"questions": [{"question": "Name?", "header": "Name", "allowOther": true,
        "options": [{"label": "Bob"}]}]});
    let questions = parse_questions(&input).unwrap();
    let mut form = QuestionForm::new(1);
    form.toggle(&questions, 0, 0);
    form.set_other(&questions, 0, "Ann".into());
    assert!(form.selections[0].is_empty());
    assert_eq!(form.answers(&questions)["Name"], "Ann");
    // Picking the option again clears the custom answer.
    form.toggle(&questions, 0, 0);
    assert_eq!(form.others[0], "");
    assert_eq!(form.answers(&questions)["Name"], "Bob");
}

#[test]
fn an_optional_free_text_form_submits_empty_answers_on_dismiss() {
    let input = json!({"questions": [{"question": "Notes?", "header": "Notes", "allowEmpty": true,
        "options": [], "dismissLabel": "Skip"}]});
    let questions = parse_questions(&input).unwrap();
    let form = QuestionForm::new(1);
    assert_eq!(dismiss_label(&questions), "Skip");
    assert_eq!(
        form.dismiss_response(&questions, &input),
        json!({"behavior": "allow", "updatedInput": {
            "questions": input["questions"], "answers": {"Notes": ""}
        }})
    );
    assert!(parse_questions(&json!({"questions": [{"header": "x"}]})).is_none());
}

#[test]
fn find_counts_matches_without_case_and_says_no_matches() {
    assert_eq!(
        match_ranges("Find the find FIND", "find"),
        [0..4, 9..13, 14..18]
    );
    // Matches do not overlap.
    assert_eq!(match_ranges("aaaa", "aa"), [0..2, 2..4]);
    assert_eq!(match_ranges("aaa", "aa").len(), 1);
    assert!(match_ranges("text", "").is_empty());
    assert_eq!(status("", 0, 0), "");
    assert_eq!(status("zzz", 0, 0), "No matches");
    assert_eq!(status("find", 3, 1), "2 of 3");
    assert_eq!(step(2, 3, true), 0);
    assert_eq!(step(0, 3, false), 2);
    assert_eq!(step(0, 0, true), 0);
}

#[test]
fn images_read_from_the_folder_that_holds_them_and_fit_480_by_360() {
    let cwd = "/tmp/gpui-bench";
    assert_eq!(
        read_target("chart.png", cwd),
        Some((cwd.into(), "chart.png".into()))
    );
    assert_eq!(
        read_target("/tmp/gpui-bench/a.png", cwd),
        Some((cwd.into(), "/tmp/gpui-bench/a.png".into()))
    );
    assert_eq!(
        read_target("/Users/x/b.png", cwd),
        Some(("/".into(), "/Users/x/b.png".into()))
    );
    assert_eq!(
        read_target("~/c.png", cwd),
        Some(("~".into(), "~/c.png".into()))
    );
    assert_eq!(read_target("rel.png", ""), None);
    // `resolveAssistantImageDisplaySize` cases from `assistant-image-metadata.test.ts`.
    assert_eq!(display_size(1200, 800), (480., 320.));
    assert_eq!(display_size(1050, 1150), (329., 360.));
    assert_eq!(display_size(100, 100), (360., 360.));
    assert_eq!(display_size(400, 100), (480., 120.));
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&640u32.to_be_bytes());
    png.extend_from_slice(&480u32.to_be_bytes());
    assert_eq!(image_dimensions(&png), Some((640, 480)));
    assert_eq!(image_dimensions(b"GIF89a\x10\x00\x20\x00"), Some((16, 32)));
    assert_eq!(image_dimensions(b"nope"), None);
    // A sent image decodes for the user bubble; a type that is not an image draws nothing.
    let sent = |mime_type: &str| SentImage {
        mime_type: mime_type.into(),
        bytes: b"hi".to_vec(),
    };
    let decoded = decode_sent_image(&sent("image/png"));
    assert_eq!(decoded.map(|image| image.bytes), Some(b"hi".to_vec()));
    assert!(decode_sent_image(&sent("text/plain")).is_none());
}
