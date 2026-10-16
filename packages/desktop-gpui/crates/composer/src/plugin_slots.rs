//! The composer slots of the native plugins (ADR-0045): pills (`plugins/buttons/view.tsx`),
//! send interceptors and composer text and chips (`plugins/composer/index.ts`), the stop button
//! face, attachment sources (`plugins/attachments`), and plugin slash commands
//! (`plugins/client-slash-commands`). Every slot reads the `ContributionIndex` of the plugin
//! host, which holds only the plugins the host reports active, so an empty index costs nothing.

use crate::pill_styles::docked_pill;
use bachuc_client::Client;
use bachuc_icons::icon;
use bachuc_plugin_host::{
    AttachmentSource, CommandContext, ComposerAttachment, ComposerPill, ComposerTarget,
    Contributed, ContributionIndex, InterceptOutcome, PillBehavior, PillPlacement, PillState,
    PluginHostEvent, PluginSlashCommand, StopButton, host,
};
use bachuc_theme::metrics::{FONT_SM, ICON_SM, RADIUS_2XL, SPACE_1, SPACE_2, SPACE_3};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Selectable, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;

/// The track pill label stops at 160 px (`button`), the toolbar and corner ones at 240.
const TRACK_LABEL_MAX: f32 = 160.;
const WIDE_LABEL_MAX: f32 = 240.;
/// `composerPillStyles.body` `minHeight` (`COMPOSER_PILL_MIN_HEIGHT`).
const FLOATING_PILL_HEIGHT: f32 = 32.;
/// `toolbarButton` height.
const TOOLBAR_PILL_HEIGHT: f32 = 28.;

/// The pills that apply to one composer, split by where they sit.
#[derive(Clone, Default)]
pub struct PillSlots {
    pub track: Vec<Contributed<ComposerPill>>,
    pub toolbar: Vec<Contributed<ComposerPill>>,
    pub corner: Vec<Contributed<ComposerPill>>,
}

/// Splits the pills that apply to this composer by `placement`, keeping registration order.
/// The desktop window is never compact here, so `placement` is used as given.
pub fn pill_slots(
    pills: &[Contributed<ComposerPill>],
    workspace_id: Option<&str>,
    agent_id: Option<&str>,
    is_draft: bool,
) -> PillSlots {
    let mut slots = PillSlots::default();
    for pill in pills {
        if !bachuc_plugin_host::pill_applies(&pill.value, workspace_id, agent_id, is_draft) {
            continue;
        }
        let slot = match pill.value.placement {
            PillPlacement::Track => &mut slots.track,
            PillPlacement::Toolbar => &mut slots.toolbar,
            PillPlacement::Corner => &mut slots.corner,
        };
        slot.push(pill.clone());
    }
    slots
}

/// What a pill press does (`pressButton`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressEffect {
    /// Runs `on_press`.
    Run,
    /// A draft opens this New workspace panel instead of running `on_press`.
    ShowPanel(&'static str),
    Menu,
    Popover,
}

pub fn press_effect(behavior: &PillBehavior, is_draft: bool) -> PressEffect {
    match behavior {
        PillBehavior::Action {
            new_workspace_panel: Some(panel),
            ..
        } if is_draft => PressEffect::ShowPanel(panel),
        PillBehavior::Action { .. } => PressEffect::Run,
        PillBehavior::Select { .. } => PressEffect::Menu,
        PillBehavior::Popover { .. } => PressEffect::Popover,
    }
}

/// The check of a select option: the option whose id is the pill value.
pub fn option_checked(value: Option<&SharedString>, option_id: &str) -> bool {
    value.is_some_and(|value| value.as_ref() == option_id)
}

/// Whether a plugin slash command shows: `Workspace` and `Agent` need a workspace, and an
/// `Agent` command needs an agent; `Global` ones always show.
pub fn command_visible(context: CommandContext, has_workspace: bool, has_agent: bool) -> bool {
    match context {
        CommandContext::Global => true,
        CommandContext::Workspace => has_workspace,
        CommandContext::Agent => has_workspace && has_agent,
    }
}

pub fn visible_commands(
    commands: &[Contributed<PluginSlashCommand>],
    has_workspace: bool,
    has_agent: bool,
) -> Vec<Contributed<PluginSlashCommand>> {
    commands
        .iter()
        .filter(|command| command_visible(command.value.context, has_workspace, has_agent))
        .cloned()
        .collect()
}

/// The stop button face: the first registered one wins.
pub fn stop_button(index: &ContributionIndex) -> Option<&Contributed<StopButton>> {
    index.stop_buttons().first()
}

/// The sources the attach menu lists: a source without a picker only opens chips added from
/// code.
pub fn picker_sources(index: &ContributionIndex) -> Vec<Contributed<AttachmentSource>> {
    index
        .attachment_sources()
        .iter()
        .filter(|source| source.value.picker.is_some())
        .cloned()
        .collect()
}

fn same_chip(left: &ComposerAttachment, right: &ComposerAttachment) -> bool {
    left.plugin_id == right.plugin_id
        && left.source_id == right.source_id
        && left.item.id == right.item.id
}

/// `togglePluginResourceAttachment`: a second pick of the same item removes its chip. Returns
/// whether the chip is now on.
pub fn toggle_chip(chips: &mut Vec<ComposerAttachment>, chip: ComposerAttachment) -> bool {
    if let Some(at) = chips.iter().position(|existing| same_chip(existing, &chip)) {
        chips.remove(at);
        return false;
    }
    chips.push(chip);
    true
}

/// `AddComposerAttachment`: adds the chip, or replaces the one for the same item in place.
pub fn upsert_chip(chips: &mut Vec<ComposerAttachment>, chip: ComposerAttachment) {
    match chips.iter().position(|existing| same_chip(existing, &chip)) {
        Some(at) => chips[at] = chip,
        None => chips.push(chip),
    }
}

/// What a host event asks of a composer for `agent_id`.
#[derive(Clone, Debug, PartialEq)]
pub enum HostAction {
    SetText(String),
    AddChip(ComposerAttachment),
    Redraw,
}

pub fn host_action(event: &PluginHostEvent, agent_id: Option<&str>) -> Option<HostAction> {
    match event {
        PluginHostEvent::ContributionsChanged | PluginHostEvent::PluginStateChanged { .. } => {
            Some(HostAction::Redraw)
        }
        PluginHostEvent::SetComposerText {
            agent_id: target,
            text,
        } if Some(target.as_str()) == agent_id => Some(HostAction::SetText(text.clone())),
        PluginHostEvent::AddComposerAttachment(chip)
            if Some(chip.agent_id.as_str()) == agent_id =>
        {
            Some(HostAction::AddChip(chip.clone()))
        }
        _ => None,
    }
}

/// With no interceptor the send runs at once, with no task.
pub fn needs_intercept(interceptors: usize) -> bool {
    interceptors > 0
}

/// `runComposerInterceptors`: runs `count` interceptors in order; each sees the text the
/// previous one returned. `None` means a `Cancel`.
pub async fn run_chain<F, Fut>(text: String, count: usize, mut step: F) -> Option<String>
where
    F: FnMut(usize, String) -> Fut,
    Fut: Future<Output = InterceptOutcome>,
{
    let mut text = text;
    for at in 0..count {
        match step(at, text.clone()).await {
            InterceptOutcome::Keep => {}
            InterceptOutcome::Replace(next) => text = next,
            InterceptOutcome::Cancel => return None,
        }
    }
    Some(text)
}

/// The views that popover pills opened, by pill, so a redraw keeps the view's state.
pub type PopoverViews = Rc<RefCell<HashMap<String, AnyView>>>;

/// A press on a draft `Action` pill with a New workspace panel.
pub type ShowPanel = Rc<dyn Fn(&'static str, &'static str, &mut App)>;

/// The visible pills of a slot with their state; a pill whose `state` is `None` hides.
pub fn pill_states(
    pills: &[Contributed<ComposerPill>],
    target: &ComposerTarget,
    cx: &App,
) -> Vec<(Contributed<ComposerPill>, PillState)> {
    pills
        .iter()
        .filter_map(|pill| (pill.value.state)(target, cx).map(|state| (pill.clone(), state)))
        .collect()
}

/// The trigger of a plugin pill in its slot.
#[derive(IntoElement)]
struct PluginPillTrigger {
    id: ElementId,
    state: PillState,
    slot: PillPlacement,
    open: bool,
}

impl Selectable for PluginPillTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for PluginPillTrigger {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        pill_body(self.id, &self.state, self.slot, self.open, cx)
    }
}

fn pill_body(
    id: ElementId,
    state: &PillState,
    slot: PillPlacement,
    open: bool,
    cx: &App,
) -> Stateful<Div> {
    let t = tokens(cx);
    let color = state.color.unwrap_or(t.foreground_muted);
    let small = ui_font_size(FONT_SM, cx);
    let label_max = match slot {
        PillPlacement::Track => TRACK_LABEL_MAX,
        PillPlacement::Toolbar | PillPlacement::Corner => WIDE_LABEL_MAX,
    };
    let body = match slot {
        PillPlacement::Track => docked_pill(id, open, cx),
        // `toolbarButton`: the agent controls' badge.
        PillPlacement::Toolbar => div()
            .id(id)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(SPACE_1))
            .h(px(TOOLBAR_PILL_HEIGHT))
            .px(px(SPACE_2))
            .mr(px(SPACE_1))
            .rounded(px(RADIUS_2XL))
            .text_size(small)
            .when(open, |pill| pill.bg(t.surface2))
            .hover(move |pill| pill.bg(t.surface2)),
        // `composerPillStyles.body`: a floating pill with its own border and fill.
        PillPlacement::Corner => div()
            .id(id)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(SPACE_2))
            .min_h(px(FLOATING_PILL_HEIGHT))
            .px(px(SPACE_3))
            .py(px(SPACE_1))
            .rounded(px(RADIUS_2XL))
            .border_1()
            .border_color(state.color.unwrap_or(t.border_accent))
            .bg(if open { t.surface2 } else { t.surface1 })
            .text_size(small)
            .hover(move |pill| pill.bg(t.surface2)),
    };
    let title = state.title.clone();
    body.role(Role::Button)
        .aria_label(title.clone())
        .text_color(color)
        .when(state.disabled, |pill| pill.opacity(0.5))
        .when(!state.disabled, |pill| pill.cursor_pointer())
        .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
        .children(
            state
                .icon
                .map(|name| icon(name).with_size(px(ICON_SM)).text_color(color)),
        )
        .children(
            state
                .label
                .clone()
                .map(|label| div().min_w_0().truncate().max_w(px(label_max)).child(label)),
        )
}

/// Draws one plugin pill with its behavior: a press runs the action (or opens a draft's New
/// workspace panel), a select opens its options with a check on the value, a popover opens the
/// plugin's view.
pub fn plugin_pill(
    pill: &Contributed<ComposerPill>,
    state: PillState,
    target: ComposerTarget,
    show_panel: ShowPanel,
    popovers: PopoverViews,
    cx: &App,
) -> AnyElement {
    let key = format!("{}/{}", pill.plugin_id, pill.value.id);
    let id = ElementId::Name(format!("plugin-pill-{key}").into());
    let slot = pill.value.placement;
    let is_draft = target.draft.is_some();
    let disabled = state.disabled;
    let effect = press_effect(&state.behavior, is_draft);
    match state.behavior.clone() {
        PillBehavior::Action { on_press, .. } => {
            let plugin_id = pill.plugin_id;
            pill_body(id, &state, slot, false, cx)
                .when(!disabled, |pill| {
                    pill.on_click(move |_, window, cx| match effect {
                        PressEffect::ShowPanel(panel) => show_panel(plugin_id, panel, cx),
                        _ => on_press(target.clone(), window, cx),
                    })
                })
                .into_any_element()
        }
        _ if disabled => pill_body(id, &state, slot, false, cx).into_any_element(),
        PillBehavior::Select {
            options,
            value,
            on_select,
        } => Popover::new(id.clone())
            .anchor(Anchor::BottomLeft)
            .p_1()
            .trigger(PluginPillTrigger {
                id: ElementId::Name(format!("plugin-pill-trigger-{key}").into()),
                state: state.clone(),
                slot,
                open: false,
            })
            .content(move |_, _, cx| {
                let t = tokens(cx);
                let rows = options.iter().enumerate().map(|(index, option)| {
                    let checked = option_checked(value.as_ref(), &option.id);
                    let (on_select, target, option_id) =
                        (on_select.clone(), target.clone(), option.id.clone());
                    crate::agent_controls::menu_row(
                        ("plugin-pill-option", index),
                        option.label.to_string(),
                        checked,
                        cx,
                    )
                    .on_click(cx.listener(move |state, _, window, cx| {
                        on_select(&target, &option_id, cx);
                        state.dismiss(window, cx);
                    }))
                    .children(option.description.clone().map(|description| {
                        div()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(t.foreground_muted)
                            .child(description)
                    }))
                });
                v_flex()
                    .id("plugin-pill-menu")
                    .role(Role::Menu)
                    .aria_label(state.title.clone())
                    .min_w(px(200.))
                    .max_h(px(320.))
                    .overflow_y_scroll()
                    .children(rows)
            })
            .into_any_element(),
        PillBehavior::Popover { flush, content } => {
            let closed = popovers.clone();
            let closed_key = key.clone();
            Popover::new(id.clone())
                .anchor(Anchor::BottomLeft)
                .map(|popover| if flush { popover.p_0() } else { popover.p_3() })
                .trigger(PluginPillTrigger {
                    id: ElementId::Name(format!("plugin-pill-trigger-{key}").into()),
                    state: state.clone(),
                    slot,
                    open: false,
                })
                .on_open_change(move |open, _, _| {
                    if !*open {
                        closed.borrow_mut().remove(&closed_key);
                    }
                })
                .content(move |_, window, cx| {
                    let cached = popovers.borrow().get(&key).cloned();
                    let view = cached.unwrap_or_else(|| {
                        let popover = cx.entity().downgrade();
                        let close: bachuc_plugin_host::PopoverClose =
                            Rc::new(move |window: &mut Window, cx: &mut App| {
                                popover
                                    .update(cx, |state, cx| state.dismiss(window, cx))
                                    .ok();
                            });
                        let view = content(target.clone(), close, window, cx);
                        popovers.borrow_mut().insert(key.clone(), view.clone());
                        view
                    });
                    div().child(view)
                })
                .into_any_element()
        }
    }
}

/// The corner pills of an agent pane: stacked at the top right, 12 px in, in registration
/// order (`PluginComposerCornerPills`).
pub struct CornerPills {
    client: Entity<Client>,
    agent_id: String,
    popovers: PopoverViews,
    _subscriptions: Vec<Subscription>,
}

impl CornerPills {
    pub fn new(
        client: Entity<Client>,
        agent_id: String,
        cx: &mut Context<CornerPills>,
    ) -> CornerPills {
        let mut subscriptions =
            vec![
                cx.subscribe(&client, |_, _, event: &bachuc_client::ClientEvent, cx| {
                    if matches!(event, bachuc_client::ClientEvent::AgentsChanged) {
                        cx.notify();
                    }
                }),
            ];
        if let Some(host) = host(cx) {
            subscriptions.push(cx.subscribe(&host, |this: &mut CornerPills, _, event, cx| {
                if host_action(event, Some(&this.agent_id)) == Some(HostAction::Redraw) {
                    cx.notify();
                }
            }));
        }
        CornerPills {
            client,
            agent_id,
            popovers: PopoverViews::default(),
            _subscriptions: subscriptions,
        }
    }
}

impl Render for CornerPills {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(host) = host(cx) else {
            return div().into_any_element();
        };
        let workspace_id = self
            .client
            .read(cx)
            .agents()
            .iter()
            .find(|agent| agent.id == self.agent_id)
            .and_then(|agent| agent.workspace_id.clone());
        let corner = pill_slots(
            host.read(cx).index().composer_pills(),
            workspace_id.as_deref(),
            Some(&self.agent_id),
            false,
        )
        .corner;
        let target = ComposerTarget {
            workspace_id,
            agent_id: Some(self.agent_id.clone()),
            draft: None,
        };
        let states = pill_states(&corner, &target, cx);
        if states.is_empty() {
            return div().into_any_element();
        }
        // A corner pill never opens a New workspace panel: it sits on an agent pane.
        let show_panel: ShowPanel = Rc::new(|_, _, _| {});
        let pills: Vec<AnyElement> = states
            .into_iter()
            .map(|(pill, state)| {
                plugin_pill(
                    &pill,
                    state,
                    target.clone(),
                    show_panel.clone(),
                    self.popovers.clone(),
                    cx,
                )
            })
            .collect();
        v_flex()
            .id("plugin-composer-corner-pills")
            .role(Role::Group)
            .aria_label("Plugin pills")
            .absolute()
            .top(px(SPACE_3))
            .right(px(SPACE_3))
            .items_end()
            .gap(px(SPACE_2))
            .children(pills)
            .into_any_element()
    }
}

/// The draft's track pills: a row above the card (`draftPills`).
pub fn draft_pill_row(pills: Vec<AnyElement>) -> Option<AnyElement> {
    if pills.is_empty() {
        return None;
    }
    Some(
        h_flex()
            .id("plugin-draft-pills")
            .role(Role::Group)
            .aria_label("Plugin pills")
            .gap(px(SPACE_1))
            .pb(px(SPACE_2))
            .children(pills)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{
        HostAction, PressEffect, command_visible, host_action, needs_intercept, option_checked,
        picker_sources, pill_slots, press_effect, run_chain, stop_button, toggle_chip, upsert_chip,
        visible_commands,
    };
    use bachuc_icons::IconName;
    use bachuc_plugin_host::{
        AttachmentItem, AttachmentPicker, AttachmentSource, CommandContext, ComposerAttachment,
        ComposerInterceptor, ComposerPill, Contributed, ContributionIndex, Contributions,
        InterceptOutcome, PillBehavior, PillPlacement, PluginHostEvent, ShowOnDraft, StopButton,
    };
    use gpui_kit::SharedString;
    use std::rc::Rc;

    fn pill(
        id: &'static str,
        placement: PillPlacement,
        show: ShowOnDraft,
    ) -> Contributed<ComposerPill> {
        Contributed {
            plugin_id: "demo",
            value: ComposerPill {
                id,
                placement,
                show_on_draft: show,
                workspace_id: None,
                agent_id: None,
                state: Rc::new(|_, _| None),
            },
        }
    }

    fn ids(pills: &[Contributed<ComposerPill>]) -> Vec<&'static str> {
        pills.iter().map(|pill| pill.value.id).collect()
    }

    fn chip(agent: &str, item: &str) -> ComposerAttachment {
        ComposerAttachment {
            agent_id: agent.into(),
            plugin_id: "demo".into(),
            source_id: "issues".into(),
            source_title: "Issues".into(),
            icon: IconName::Github,
            item: AttachmentItem {
                id: item.into(),
                identifier: format!("#{item}"),
                title: "Title".into(),
                subtitle: None,
                url: String::new(),
                text: "body".into(),
                resource_type: "issue".into(),
            },
            commentable: false,
        }
    }

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        futures::executor::block_on(future)
    }

    #[test]
    fn slots_split_by_placement_in_order() {
        let pills = vec![
            pill("a", PillPlacement::Track, ShowOnDraft::No),
            pill("b", PillPlacement::Toolbar, ShowOnDraft::Yes),
            pill("c", PillPlacement::Corner, ShowOnDraft::No),
            pill("d", PillPlacement::Track, ShowOnDraft::Only),
            pill("e", PillPlacement::Track, ShowOnDraft::Yes),
        ];
        let agent = pill_slots(&pills, Some("w"), Some("x"), false);
        assert_eq!(ids(&agent.track), ["a", "e"]);
        assert_eq!(ids(&agent.toolbar), ["b"]);
        assert_eq!(ids(&agent.corner), ["c"]);
        let draft = pill_slots(&pills, None, None, true);
        assert_eq!(ids(&draft.track), ["d", "e"]);
        assert_eq!(ids(&draft.toolbar), ["b"]);
        assert!(draft.corner.is_empty());
    }

    #[test]
    fn a_pill_scoped_to_another_agent_is_hidden() {
        let mut scoped = pill("s", PillPlacement::Track, ShowOnDraft::No);
        scoped.value.agent_id = Some("other".into());
        assert!(
            pill_slots(&[scoped.clone()], Some("w"), Some("x"), false)
                .track
                .is_empty()
        );
        assert_eq!(
            ids(&pill_slots(&[scoped], Some("w"), Some("other"), false).track),
            ["s"]
        );
    }

    #[test]
    fn a_draft_panel_action_opens_the_panel_and_not_on_press() {
        let action = PillBehavior::Action {
            on_press: Rc::new(|_, _, _| panic!("on_press must not run")),
            new_workspace_panel: Some("routes"),
        };
        assert_eq!(
            press_effect(&action, true),
            PressEffect::ShowPanel("routes")
        );
        assert_eq!(press_effect(&action, false), PressEffect::Run);
        let select = PillBehavior::Select {
            options: Vec::new(),
            value: None,
            on_select: Rc::new(|_, _, _| {}),
        };
        assert_eq!(press_effect(&select, true), PressEffect::Menu);
    }

    #[test]
    fn a_select_checks_its_value() {
        let value = Some(SharedString::from("fast"));
        assert!(option_checked(value.as_ref(), "fast"));
        assert!(!option_checked(value.as_ref(), "slow"));
        assert!(!option_checked(None, "fast"));
    }

    #[test]
    fn interceptors_run_in_order_on_the_previous_text() {
        let seen = std::cell::RefCell::new(Vec::new());
        let outcomes = [
            InterceptOutcome::Replace("b".into()),
            InterceptOutcome::Keep,
        ];
        let sent = block_on(run_chain("a".into(), 2, |at, text| {
            seen.borrow_mut().push(text);
            let outcome = outcomes[at].clone();
            async move { outcome }
        }));
        assert_eq!(sent.as_deref(), Some("b"));
        assert_eq!(*seen.borrow(), ["a", "b"]);
        let cancelled = block_on(run_chain("a".into(), 2, |_, _| async {
            InterceptOutcome::Cancel
        }));
        assert_eq!(cancelled, None);
    }

    #[test]
    fn the_same_item_picked_twice_leaves_no_chip() {
        let mut chips = Vec::new();
        assert!(toggle_chip(&mut chips, chip("x", "1")));
        assert!(!toggle_chip(&mut chips, chip("x", "1")));
        assert!(chips.is_empty());
        upsert_chip(&mut chips, chip("x", "1"));
        let mut changed = chip("x", "1");
        changed.commentable = true;
        upsert_chip(&mut chips, changed);
        assert_eq!(chips.len(), 1);
        assert!(chips[0].commentable);
    }

    #[test]
    fn host_events_for_another_agent_are_ignored() {
        let text = PluginHostEvent::SetComposerText {
            agent_id: "other".into(),
            text: "hi".into(),
        };
        assert_eq!(host_action(&text, Some("x")), None);
        let mine = PluginHostEvent::SetComposerText {
            agent_id: "x".into(),
            text: "hi".into(),
        };
        assert_eq!(
            host_action(&mine, Some("x")),
            Some(HostAction::SetText("hi".into()))
        );
        let add = PluginHostEvent::AddComposerAttachment(chip("other", "1"));
        assert_eq!(host_action(&add, Some("x")), None);
        let add_mine = PluginHostEvent::AddComposerAttachment(chip("x", "1"));
        assert_eq!(
            host_action(&add_mine, Some("x")),
            Some(HostAction::AddChip(chip("x", "1")))
        );
        assert_eq!(host_action(&mine, None), None);
        assert_eq!(
            host_action(&PluginHostEvent::ContributionsChanged, None),
            Some(HostAction::Redraw)
        );
    }

    #[test]
    fn command_context_needs_a_workspace_or_an_agent() {
        assert!(command_visible(CommandContext::Global, false, false));
        assert!(!command_visible(CommandContext::Workspace, false, false));
        assert!(command_visible(CommandContext::Workspace, true, false));
        assert!(!command_visible(CommandContext::Agent, true, false));
        assert!(command_visible(CommandContext::Agent, true, true));
    }

    #[test]
    fn with_no_active_plugin_the_slots_are_empty() {
        let index = ContributionIndex::default();
        let slots = pill_slots(index.composer_pills(), Some("w"), Some("x"), false);
        assert!(slots.track.is_empty() && slots.toolbar.is_empty() && slots.corner.is_empty());
        assert!(stop_button(&index).is_none());
        assert!(picker_sources(&index).is_empty());
        assert!(visible_commands(index.slash_commands(), true, true).is_empty());
        assert!(index.composer_interceptors().is_empty());
    }

    /// A fake plugin with a track pill, a toolbar pill, an interceptor, two stop buttons, and
    /// an attachment source with a picker.
    fn fake_plugin() -> Contributions {
        let mut plugin = Contributions::default();
        plugin.add_composer_pill(pill("track", PillPlacement::Track, ShowOnDraft::No).value);
        plugin.add_composer_pill(pill("toolbar", PillPlacement::Toolbar, ShowOnDraft::No).value);
        plugin.add_composer_interceptor(ComposerInterceptor {
            id: "route",
            intercept: Rc::new(|_, _| unreachable!("not run in this test")),
        });
        for id in ["first", "second"] {
            plugin.add_stop_button(StopButton {
                id,
                render: Rc::new(|_, _, _| unreachable!("not drawn in this test")),
            });
        }
        plugin.add_attachment_source(AttachmentSource {
            id: "issues",
            title: "Issues".into(),
            icon: IconName::Github,
            picker: Some(AttachmentPicker {
                title: "Issues".into(),
                search_placeholder: "Search issues".into(),
                search_method: "issues.search",
            }),
            on_open: None,
        });
        plugin.add_attachment_source(AttachmentSource {
            id: "code-only",
            title: "Code only".into(),
            icon: IconName::Github,
            picker: None,
            on_open: None,
        });
        plugin
    }

    #[test]
    fn an_active_fake_plugin_fills_each_slot_and_an_inactive_one_none() {
        let plugin = fake_plugin();
        let active = ContributionIndex::build([("demo", &plugin)]);
        let slots = pill_slots(active.composer_pills(), Some("w"), Some("x"), false);
        assert_eq!(ids(&slots.track), ["track"]);
        assert_eq!(ids(&slots.toolbar), ["toolbar"]);
        assert_eq!(active.composer_interceptors().len(), 1);
        // Two stop buttons: the first one draws.
        assert_eq!(
            stop_button(&active).map(|button| button.value.id),
            Some("first")
        );
        let sources = picker_sources(&active);
        assert_eq!(
            sources
                .iter()
                .map(|source| source.value.id)
                .collect::<Vec<_>>(),
            ["issues"]
        );
        // Inactive: the host leaves the plugin out of the index.
        let inactive = ContributionIndex::build(std::iter::empty());
        assert!(
            pill_slots(inactive.composer_pills(), Some("w"), Some("x"), false)
                .track
                .is_empty()
        );
        assert!(stop_button(&inactive).is_none());
    }

    #[test]
    fn a_send_with_no_interceptor_starts_no_task() {
        assert!(!needs_intercept(0));
        assert!(needs_intercept(1));
    }
}
