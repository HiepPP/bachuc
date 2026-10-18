//! The workspace draft tab, after Electron's `composer/draft/workspace-tab.tsx`: a draft
//! composer docked at the bottom of a tab, in the workspace folder. A submit creates the agent
//! with the draft's full config through `Client::create_agent_with`, and the owner replaces the
//! tab with the agent tab.

use crate::new_workspace::{SELECT_MODEL, SubmitGuard};
use bachuc_client::{Client, InitialAgent};
use bachuc_composer::{
    Composer, ComposerEvent, DraftOptions, DraftSeed, DraftSubmission, SubmitIcon, composer_dock,
};
use bachuc_theme::metrics::{FONT_BASE, SPACE_2, SPACE_4};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::v_flex;
use gpui_kit::*;
use std::collections::BTreeMap;

/// The draft made an agent; the owner replaces the draft tab with the agent tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created {
    pub key: String,
    pub agent_id: String,
}

/// The `create_agent` request of a submit: the draft's config, feature values, plugin
/// environment, images, and attachments.
pub fn draft_agent(submission: &DraftSubmission) -> Result<InitialAgent, String> {
    let Some(config) = submission.config.clone() else {
        return Err(SELECT_MODEL.to_string());
    };
    Ok(InitialAgent {
        config,
        feature_values: submission.feature_values.clone(),
        env: submission.environment.clone(),
        initial_prompt: submission.text.clone(),
        images: submission.images.clone(),
        attachments: submission.attachments.clone(),
        labels: BTreeMap::new(),
    })
}

pub struct WorkspaceDraftPane {
    client: Entity<Client>,
    key: String,
    composer: Entity<Composer>,
    guard: SubmitGuard,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Created> for WorkspaceDraftPane {}

impl WorkspaceDraftPane {
    pub fn new(
        client: Entity<Client>,
        key: String,
        cwd: String,
        window: &mut Window,
        cx: &mut Context<WorkspaceDraftPane>,
    ) -> WorkspaceDraftPane {
        let options = DraftOptions {
            cwd: Some(cwd),
            min_lines: 1,
            submit_icon: SubmitIcon::Arrow,
            allow_empty_submit: false,
        };
        let composer =
            cx.new(|cx| Composer::new_draft(client.clone(), key.clone(), options, window, cx));
        let subscriptions = vec![
            cx.subscribe_in(
                &composer,
                window,
                |this: &mut WorkspaceDraftPane, _, event: &ComposerEvent, _, cx| {
                    if *event == ComposerEvent::DraftSubmit {
                        this.submit(cx);
                    }
                },
            ),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        WorkspaceDraftPane {
            client,
            key,
            composer,
            guard: SubmitGuard::default(),
            error: None,
            _subscriptions: subscriptions,
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.composer
            .update(cx, |composer, cx| composer.focus(window, cx));
    }

    /// A fork draft: the source agent's settings, when it has a provider, and the chat history
    /// chip.
    pub fn seed_fork(
        &mut self,
        seed: Option<DraftSeed>,
        label: String,
        attachment: serde_json::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.composer.update(cx, |composer, cx| {
            if let Some(seed) = seed {
                composer.set_draft_seed(seed, window, cx);
            }
            composer.add_raw_attachment(label, attachment, cx);
        });
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let Some(submission) = self.composer.read(cx).draft_submission(cx) else {
            return;
        };
        if !self.guard.begin() {
            return;
        }
        let agent = match draft_agent(&submission) {
            Ok(agent) => agent,
            Err(message) => {
                self.guard.finish();
                self.error = Some(message);
                cx.notify();
                return;
            }
        };
        self.error = None;
        self.composer
            .update(cx, |composer, cx| composer.set_submitting(true, cx));
        let task = self
            .client
            .update(cx, |client, cx| client.create_agent_with(agent, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.guard.finish();
                this.composer
                    .update(cx, |composer, cx| composer.set_submitting(false, cx));
                match result {
                    Ok(agent_id) => cx.emit(Created {
                        key: this.key.clone(),
                        agent_id,
                    }),
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Render for WorkspaceDraftPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let content = v_flex()
            .w_full()
            .px(zoomed(SPACE_4, cx))
            .pb(zoomed(SPACE_2, cx))
            .children(self.error.clone().map(|message| {
                div()
                    .id("workspace-draft-error")
                    .role(Role::Alert)
                    .aria_label(format!("Error: {message}"))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(t.destructive)
                    .child(message)
            }))
            .into_any_element();
        div()
            .id("pane-workspace-draft")
            .role(Role::Group)
            .aria_label("New agent")
            .size_full()
            .bg(t.surface0)
            .child(composer_dock(
                false,
                content,
                self.composer.clone().into_any_element(),
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::draft_agent;
    use crate::new_workspace::SELECT_MODEL;
    use bachuc_client::DraftConfig;
    use bachuc_composer::DraftSubmission;
    use serde_json::json;

    #[test]
    fn the_request_carries_the_mode_thinking_features_env_and_files() {
        let submission = DraftSubmission {
            text: "hello".into(),
            images: vec![json!({"mimeType": "image/png"})],
            attachments: vec![json!({"type": "plugin"})],
            config: Some(DraftConfig {
                provider: "codex".into(),
                cwd: "/repo/wt".into(),
                model: Some("gpt".into()),
                mode_id: Some("full-access".into()),
                thinking_option_id: Some("high".into()),
            }),
            feature_values: [("fast".to_string(), json!(true))].into(),
            environment: [("PLUGIN_X".to_string(), "on".to_string())].into(),
        };
        let agent = draft_agent(&submission).unwrap();
        assert_eq!(agent.config.cwd, "/repo/wt");
        assert_eq!(agent.config.mode_id.as_deref(), Some("full-access"));
        assert_eq!(agent.config.thinking_option_id.as_deref(), Some("high"));
        assert_eq!(agent.feature_values["fast"], json!(true));
        assert_eq!(agent.env["PLUGIN_X"], "on");
        assert_eq!(agent.initial_prompt, "hello");
        assert_eq!(agent.images.len(), 1);
        assert_eq!(agent.attachments.len(), 1);
    }

    #[test]
    fn a_draft_without_a_model_asks_for_one() {
        let submission = DraftSubmission {
            text: "hello".into(),
            ..DraftSubmission::default()
        };
        assert_eq!(draft_agent(&submission).unwrap_err(), SELECT_MODEL);
    }
}
