//! The Delegated work panel (`client/panel.tsx`): the jobs of one agent, polled every 5 s while
//! the view lives. The rules are in `model`; this file sends the requests and draws the rows.

use crate::model::{
    AttemptView, CANCEL_TEXT, CancelInput, Cancelled, EMPTY_TEXT, FAILED_TEXT, HINT_TEXT,
    JOBS_CANCEL, JOBS_LIST, JobView, JobsItem, JobsPayload, JobsState, LOADING_TEXT, Poller,
    REFRESH_TEXT, ScopeInput, TITLE_TEXT, attempt_child_line, attempt_output, attempt_title,
    cancellable, check_text, decode_jobs, job_title, keyed_checks,
};
use bachuc_client::{Client, ClientEvent};
use bachuc_plugin_host::{PanelInput, PluginContext};
use bachuc_theme::metrics::{
    FONT_2XL, FONT_BASE, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3, WEIGHT_SEMIBOLD,
};
use bachuc_theme::{Tokens, tokens, ui_font_size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Electron's `padding: compact ? 12 : 20`, on desktop.
const CONTENT_PADDING: f32 = 20.;
const CONTENT_GAP: f32 = 16.;
const BUTTON_PADDING: f32 = 10.;
const TEXT_LINE_HEIGHT: f32 = 1.2;
const OVERDRAW: f32 = 400.;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

pub(crate) fn build(ctx: &PluginContext, input: PanelInput, cx: &mut App) -> AnyView {
    let ctx = ctx.clone();
    let agent_id = input.agent_id.unwrap_or_default();
    cx.new(|cx| JobsPanel::new(ctx, agent_id, cx)).into()
}

pub(crate) struct JobsPanel {
    ctx: PluginContext,
    agent_id: String,
    state: JobsState,
    poller: Poller,
    items: Vec<JobsItem>,
    list: ListState,
    /// Held strong: a subscription holds its emitter weakly.
    _client: Option<Entity<Client>>,
    _subscription: Option<Subscription>,
    // Dropping the view drops these, which stops the timer and cancels the request.
    _poll: Option<Task<()>>,
    _request: Option<Task<()>>,
    _cancel: Option<Task<()>>,
}

impl JobsPanel {
    fn new(ctx: PluginContext, agent_id: String, cx: &mut Context<Self>) -> Self {
        let client: Option<Entity<Client>> = ctx.client(cx);
        let connected = client.as_ref().is_some_and(|c| c.read(cx).is_connected());
        let mut poller = Poller::default();
        let mut state = JobsState::default();
        if connected {
            poller.connected();
        } else {
            // A request sent before `Connected` fails and is not retried, so the panel shows its
            // error and loads again on the next `Connected`.
            state.list_failed = true;
        }
        let subscription = client.as_ref().map(|client| {
            cx.subscribe(client, |panel: &mut Self, _, event: &ClientEvent, cx| {
                match event {
                    ClientEvent::Connected => panel.poller.connected(),
                    ClientEvent::Disconnected { .. } => {
                        panel.poller.disconnected();
                        panel.state.list_failed = true;
                    }
                    _ => return,
                }
                panel.sync(cx);
                panel.schedule(cx);
            })
        });
        let mut panel = Self {
            ctx,
            agent_id,
            state,
            poller,
            items: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            _client: client,
            _subscription: subscription,
            _poll: None,
            _request: None,
            _cancel: None,
        };
        panel.sync(cx);
        panel.schedule(cx);
        panel
    }

    /// Sends when a poll is due, else sleeps until the next one.
    fn schedule(&mut self, cx: &mut Context<Self>) {
        let now = now_ms();
        if self.poller.due(now) {
            self.send(now, cx);
            return;
        }
        self._poll = self.poller.wait_ms(now).map(|delay| {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(delay.max(0) as u64))
                    .await;
                this.update(cx, |panel, cx| panel.schedule(cx)).ok();
            })
        });
    }

    fn send(&mut self, now: i64, cx: &mut Context<Self>) {
        self.poller.begin(now);
        self._poll = None;
        let input = ScopeInput {
            parent_id: self.agent_id.clone(),
        };
        let answer = self.ctx.rpc_as::<_, JobsPayload>(JOBS_LIST, &input, cx);
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = answer.await;
            this.update(cx, |panel, cx| panel.received(result, cx)).ok();
        }));
    }

    fn received(&mut self, result: anyhow::Result<JobsPayload>, cx: &mut Context<Self>) {
        match result {
            Ok(payload) => self.state.listed(Ok(decode_jobs(payload))),
            Err(error) => {
                log::warn!("jev-orchestrator: jobs.list failed: {error:#}");
                self.state.listed(Err(()));
            }
        }
        let resend = self.poller.finish();
        self.sync(cx);
        if resend {
            self.send(now_ms(), cx);
        } else {
            self.schedule(cx);
        }
    }

    /// The Refresh button and a finished cancel: skip the interval, never the request in flight.
    fn refetch(&mut self, cx: &mut Context<Self>) {
        if self.poller.request_now() {
            self.send(now_ms(), cx);
        }
    }

    fn cancel(&mut self, id: String, cx: &mut Context<Self>) {
        if !self.state.begin_cancel() {
            return;
        }
        self.sync(cx);
        let input = CancelInput {
            parent_id: self.agent_id.clone(),
            id,
        };
        let answer = self.ctx.rpc_as::<_, Cancelled>(JOBS_CANCEL, &input, cx);
        self._cancel = Some(cx.spawn(async move |this, cx| {
            let result = answer.await;
            this.update(cx, |panel, cx| panel.cancel_done(result.is_ok(), cx))
                .ok();
        }));
    }

    fn cancel_done(&mut self, ok: bool, cx: &mut Context<Self>) {
        self.state.cancel_finished(ok);
        self.sync(cx);
        if ok {
            self.refetch(cx);
        }
    }

    /// Measures the rows again only when the rows differ; a poll with the same jobs draws on.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let items = self.state.items();
        if items != self.items {
            self.list.splice(0..self.items.len(), items.len());
            self.items = items;
        }
        cx.notify();
    }

    fn render_row(&mut self, index: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(item) = self.items.get(index).cloned() else {
            return div().into_any_element();
        };
        let colors = tokens(cx);
        let last = index + 1 == self.items.len();
        let body = match &item {
            JobsItem::Title => div()
                .id("jev-jobs-title")
                .role(Role::Heading)
                .aria_label(TITLE_TEXT)
                .text_size(ui_font_size(FONT_2XL, cx))
                .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                .text_color(colors.foreground)
                .child(TITLE_TEXT)
                .into_any_element(),
            JobsItem::Hint => div()
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(colors.foreground_muted)
                .child(HINT_TEXT)
                .into_any_element(),
            JobsItem::Failed => div()
                .id("jev-jobs-failed")
                .role(Role::Alert)
                .aria_label(FAILED_TEXT)
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(colors.foreground)
                .child(FAILED_TEXT)
                .into_any_element(),
            JobsItem::Loading => plain_text(LOADING_TEXT, colors, cx),
            JobsItem::Empty => plain_text(EMPTY_TEXT, colors, cx),
            JobsItem::Job(job) => self.job_card(job, colors, cx),
            JobsItem::Refresh => div()
                .id("jev-jobs-refresh")
                .role(Role::Button)
                .aria_label("Refresh delegated jobs")
                .p(px(BUTTON_PADDING))
                .rounded(px(RADIUS_MD))
                .bg(colors.surface1)
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(colors.foreground)
                .cursor_pointer()
                .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| panel.refetch(cx)))
                .child(REFRESH_TEXT)
                .into_any_element(),
        };
        div()
            .w_full()
            .px(px(CONTENT_PADDING))
            .pt(px(if index == 0 { CONTENT_PADDING } else { 0. }))
            .pb(px(if last { CONTENT_PADDING } else { CONTENT_GAP }))
            .line_height(relative(TEXT_LINE_HEIGHT))
            .child(body)
            .into_any_element()
    }

    fn job_card(&self, job: &JobView, colors: Tokens, cx: &mut Context<Self>) -> AnyElement {
        let size = ui_font_size(FONT_BASE, cx);
        let title = job_title(job);
        let id = job.task.id.clone();
        let disabled = self.state.cancelling;
        div()
            .id(SharedString::from(format!("jev-job-{id}")))
            .role(Role::Group)
            .aria_label(title.clone())
            .flex()
            .flex_col()
            .gap(px(SPACE_2))
            .p(px(SPACE_3))
            .border_1()
            .border_color(colors.border)
            .rounded(px(RADIUS_LG))
            .text_size(size)
            .text_color(colors.foreground)
            .child(div().font_weight(FontWeight(WEIGHT_SEMIBOLD)).child(title))
            .child(job.message.clone())
            .children(job.notify_error.clone())
            .children(
                job.attempts
                    .iter()
                    .map(|attempt| attempt_card(attempt, colors)),
            )
            .when(cancellable(&job.status), |card| {
                card.child(
                    div()
                        .id(SharedString::from(format!("jev-cancel-{id}")))
                        .role(Role::Button)
                        .aria_label(format!("Cancel {id}"))
                        .p(px(BUTTON_PADDING))
                        .rounded(px(RADIUS_MD))
                        .bg(colors.surface1)
                        .when(disabled, |button| button.aria_description("Disabled"))
                        .when(!disabled, |button| {
                            button.cursor_pointer().on_click(cx.listener(
                                move |panel, _: &ClickEvent, _, cx| panel.cancel(id.clone(), cx),
                            ))
                        })
                        .child(CANCEL_TEXT),
                )
            })
            .into_any_element()
    }
}

fn plain_text(text: &'static str, colors: Tokens, cx: &App) -> AnyElement {
    div()
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(colors.foreground)
        .child(text)
        .into_any_element()
}

fn attempt_card(attempt: &AttemptView, colors: Tokens) -> Div {
    let checks = attempt
        .checks
        .as_deref()
        .map(keyed_checks)
        .unwrap_or_default();
    div()
        .flex()
        .flex_col()
        .gap(px(SPACE_1))
        .child(div().child(attempt_title(attempt)))
        .child(
            div()
                .text_color(colors.foreground_muted)
                .child(attempt_child_line(attempt)),
        )
        .children(
            checks
                .iter()
                .map(|keyed| div().child(check_text(keyed.number, keyed.check))),
        )
        .children(attempt_output(attempt).map(|output| div().child(output.to_string())))
}

impl Render for JobsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        div()
            .id("jev-jobs-panel")
            .role(Role::Group)
            .aria_label(TITLE_TEXT)
            .size_full()
            .bg(colors.surface0)
            .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full())
    }
}
