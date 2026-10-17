//! Permission cards: the plan card, the action buttons, and the question form, as
//! `PermissionRequestCard` in `packages/app/src/agent-stream/view.tsx`, `plan-card.tsx`, and
//! `question-form-card-core.ts`. Each answer is the `AgentPermissionResponse` the app sends.

use serde_json::{Map, Value, json};

/// One action button of a permission card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub allow: bool,
    pub primary: bool,
}

pub fn is_plan(request: &Value) -> bool {
    request["kind"] == "plan"
}

pub fn is_question(request: &Value) -> bool {
    request["kind"] == "question"
}

/// The card title: "Plan" for a plan, else the request title or name.
pub fn title(request: &Value) -> String {
    if is_plan(request) {
        return "Plan".into();
    }
    request["title"]
        .as_str()
        .or_else(|| request["name"].as_str())
        .unwrap_or("Permission Required")
        .to_string()
}

/// The plan Markdown: `metadata.planText`, else `input.plan`.
pub fn plan_text(request: &Value) -> Option<String> {
    request["metadata"]["planText"]
        .as_str()
        .filter(|text| !text.is_empty())
        .or_else(|| request["input"]["plan"].as_str())
        .map(str::to_string)
}

/// The request's own actions, or Deny and Accept (Implement for a plan).
pub fn actions(request: &Value) -> Vec<Action> {
    if is_question(request) {
        return Vec::new();
    }
    if let Some(list) = request["actions"]
        .as_array()
        .filter(|list| !list.is_empty())
    {
        return list
            .iter()
            .map(|action| Action {
                id: action["id"].as_str().unwrap_or("").to_string(),
                label: action["label"].as_str().unwrap_or("").to_string(),
                allow: action["behavior"] == "allow",
                primary: action["variant"] == "primary",
            })
            .collect();
    }
    vec![
        Action {
            id: "reject".into(),
            label: "Deny".into(),
            allow: false,
            primary: false,
        },
        Action {
            id: "accept".into(),
            label: if is_plan(request) {
                "Implement"
            } else {
                "Accept"
            }
            .into(),
            allow: true,
            primary: true,
        },
    ]
}

/// The response to an action button.
pub fn action_response(action: &Action) -> Value {
    if action.allow {
        json!({"behavior": "allow", "selectedActionId": action.id})
    } else {
        json!({"behavior": "deny", "selectedActionId": action.id, "message": "Denied by user"})
    }
}

/// The outcome a finished plan tool call shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanOutcome {
    Pending,
    Approved,
    Rejected,
    Canceled,
}

/// `resolvePlanOutcome` of a plan tool call item.
pub fn plan_outcome(item: &Value) -> Option<PlanOutcome> {
    let status = item["status"].as_str().unwrap_or("");
    if status == "canceled" {
        return Some(PlanOutcome::Canceled);
    }
    match item["metadata"]["approved"].as_bool() {
        Some(false) => return Some(PlanOutcome::Rejected),
        Some(true) => return Some(PlanOutcome::Approved),
        None => {}
    }
    matches!(status, "running" | "executing").then_some(PlanOutcome::Pending)
}

pub fn plan_title(outcome: Option<PlanOutcome>, pending: &str) -> String {
    match outcome {
        Some(PlanOutcome::Rejected) => "Rejected plan".into(),
        Some(PlanOutcome::Approved) => "Approved plan".into(),
        Some(PlanOutcome::Canceled) => "Canceled plan".into(),
        _ => pending.into(),
    }
}

/// A rejected or canceled plan starts folded.
pub fn plan_starts_open(outcome: Option<PlanOutcome>) -> bool {
    !matches!(outcome, Some(PlanOutcome::Rejected | PlanOutcome::Canceled))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionOption {
    pub label: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    pub multi_select: bool,
    pub allow_other: bool,
    pub allow_empty: bool,
    pub placeholder: Option<String>,
    pub dismiss_label: Option<String>,
}

impl Question {
    /// `questionShowsTextInput`.
    pub fn shows_text_input(&self) -> bool {
        self.options.is_empty() || self.allow_other
    }
}

/// `parseQuestionFormQuestions`: None when any question is malformed.
pub fn parse_questions(input: &Value) -> Option<Vec<Question>> {
    let raw = input["questions"].as_array()?;
    let mut questions = Vec::new();
    for item in raw {
        let question = item["question"].as_str()?;
        let header = item["header"].as_str()?;
        let mut options = Vec::new();
        for option in item["options"].as_array()? {
            options.push(QuestionOption {
                label: option["label"].as_str()?.to_string(),
                description: option["description"].as_str().map(str::to_string),
            });
        }
        questions.push(Question {
            question: question.to_string(),
            header: header.to_string(),
            options,
            multi_select: item["multiSelect"] == true,
            allow_other: item["allowOther"] == true || item["isOther"] == true,
            allow_empty: item["allowEmpty"] == true,
            placeholder: item["placeholder"].as_str().map(str::to_string),
            dismiss_label: item["dismissLabel"].as_str().map(str::to_string),
        });
    }
    (!questions.is_empty()).then_some(questions)
}

/// The answers of one question form while the user fills it in.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct QuestionForm {
    /// The picked options of each question, in the order they were picked, as the app's `Set`.
    pub selections: Vec<Vec<usize>>,
    pub others: Vec<String>,
    pub active: usize,
}

impl QuestionForm {
    pub fn new(count: usize) -> QuestionForm {
        QuestionForm {
            selections: vec![Vec::new(); count],
            others: vec![String::new(); count],
            active: 0,
        }
    }

    /// `toggleOption`. Single select: an option replaces the custom answer and moves on to the
    /// next question.
    pub fn toggle(&mut self, questions: &[Question], question: usize, option: usize) {
        let multi = questions[question].multi_select;
        let selected = &mut self.selections[question];
        let was_selected = selected.contains(&option);
        if multi {
            if was_selected {
                selected.retain(|picked| *picked != option);
            } else {
                selected.push(option);
            }
        } else {
            selected.clear();
            if !was_selected {
                selected.push(option);
            }
        }
        if !multi {
            self.others[question].clear();
            if !self.selections[question].is_empty() && question == self.active {
                self.active = (question + 1).min(questions.len() - 1);
            }
        }
    }

    /// `setOtherText`. Single select: a custom answer clears the option.
    pub fn set_other(&mut self, questions: &[Question], question: usize, text: String) {
        if !questions[question].multi_select && !text.is_empty() {
            self.selections[question].clear();
        }
        self.others[question] = text;
    }

    /// `isQuestionAnswered`.
    pub fn is_answered(&self, questions: &[Question], question: usize) -> bool {
        if !self.selections[question].is_empty() {
            return true;
        }
        let q = &questions[question];
        if !q.shows_text_input() {
            return false;
        }
        !self.others[question].trim().is_empty() || q.allow_empty
    }

    pub fn all_answered(&self, questions: &[Question]) -> bool {
        (0..questions.len()).all(|index| self.is_answered(questions, index))
    }

    pub fn is_last(&self, questions: &[Question]) -> bool {
        self.active + 1 >= questions.len()
    }

    /// Whether the primary button (Next or Submit) is enabled.
    pub fn can_advance(&self, questions: &[Question]) -> bool {
        if self.is_last(questions) {
            self.all_answered(questions)
        } else {
            self.is_answered(questions, self.active)
        }
    }

    /// `buildQuestionFormAnswers`, keyed by question header.
    pub fn answers(&self, questions: &[Question]) -> Map<String, Value> {
        let mut answers = Map::new();
        for (index, q) in questions.iter().enumerate() {
            let other = self.others[index].trim();
            let labels: Vec<&str> = self.selections[index]
                .iter()
                .map(|&option| q.options[option].label.as_str())
                .collect();
            if q.shows_text_input() {
                if !other.is_empty() {
                    let answer = if q.multi_select {
                        labels
                            .iter()
                            .copied()
                            .chain([other])
                            .collect::<Vec<_>>()
                            .join(", ")
                    } else {
                        other.to_string()
                    };
                    answers.insert(q.header.clone(), answer.into());
                    continue;
                }
                if q.allow_empty && q.options.is_empty() {
                    answers.insert(q.header.clone(), "".into());
                    continue;
                }
            }
            if !labels.is_empty() {
                answers.insert(q.header.clone(), labels.join(", ").into());
            }
        }
        answers
    }

    /// The Submit response: the request input with the answers added.
    pub fn submit_response(&self, questions: &[Question], input: &Value) -> Value {
        let mut updated = input.as_object().cloned().unwrap_or_default();
        updated.insert("answers".into(), Value::Object(self.answers(questions)));
        json!({"behavior": "allow", "updatedInput": updated})
    }

    /// The Dismiss response. A form of only optional free-text questions submits empty answers.
    pub fn dismiss_response(&self, questions: &[Question], input: &Value) -> Value {
        let submit_empty = questions
            .iter()
            .all(|q| q.allow_empty && q.options.is_empty());
        if submit_empty {
            self.submit_response(questions, input)
        } else {
            json!({"behavior": "deny", "message": "Dismissed by user"})
        }
    }
}

/// `resolveDismissLabel`.
pub fn dismiss_label(questions: &[Question]) -> String {
    questions
        .iter()
        .find_map(|q| q.dismiss_label.clone())
        .unwrap_or_else(|| "Dismiss".into())
}

/// The placeholder of a question's text field.
pub fn placeholder(question: &Question) -> String {
    question.placeholder.clone().unwrap_or_else(|| {
        if question.options.is_empty() {
            "Type your answer...".into()
        } else {
            "Other...".into()
        }
    })
}
