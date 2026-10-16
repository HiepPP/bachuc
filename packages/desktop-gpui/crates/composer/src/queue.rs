//! The queued prompts above the composer (`renderQueueTrack` and `QueuedMessageRow` in
//! `packages/app/src/composer/index.tsx`, and `daemon-queue.ts`): Move up, Move down, Cancel,
//! Edit, and Send now per row, and the held notice with Resume.

use bachuc_client::QueuedPrompt;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_BASE, ICON_SM, RADIUS_LG, SPACE_2, SPACE_3};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::*;
use std::rc::Rc;

/// `queueActionButton`.
const ACTION_SIZE: f32 = 32.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// `daemonQueueMoveTarget`: the `before_id` that moves an item one step, `Some(None)` for the
/// end, or `None` when it cannot move.
pub fn move_target(
    items: &[QueuedPrompt],
    id: &str,
    direction: Direction,
) -> Option<Option<String>> {
    let index = items.iter().position(|item| item.id == id)?;
    match direction {
        Direction::Up => {
            let previous = items.get(index.checked_sub(1)?)?;
            Some(Some(previous.id.clone()))
        }
        Direction::Down => {
            if index + 1 >= items.len() {
                return None;
            }
            Some(items.get(index + 2).map(|item| item.id.clone()))
        }
    }
}

/// Whether a row shows Move up and Move down: the first row cannot move up, the last cannot
/// move down.
pub fn row_moves(index: usize, len: usize) -> (bool, bool) {
    (index > 0, index + 1 < len)
}

/// What a row press does. The composer owns each.
#[derive(Clone)]
pub enum QueueAction {
    Move(String, Direction),
    Cancel(String),
    Edit(String),
    SendNow(String),
    Resume,
}

type OnAction = Rc<dyn Fn(QueueAction, &mut Window, &mut App)>;

fn action_button(
    id: impl Into<ElementId>,
    glyph: IconName,
    label: &'static str,
    primary: bool,
    action: QueueAction,
    on_action: &OnAction,
    cx: &App,
) -> AnyElement {
    let t = tokens(cx);
    let on_action = on_action.clone();
    let (background, hover, foreground) = if primary {
        (t.accent, t.accent_bright, t.accent_foreground)
    } else {
        (t.surface2, t.surface3, t.foreground)
    };
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(ACTION_SIZE))
        .rounded_full()
        .cursor_pointer()
        .bg(background)
        .hover(move |button| button.bg(hover))
        .on_click(move |_, window, cx| on_action(action.clone(), window, cx))
        .child(icon(glyph).with_size(px(ICON_SM)).text_color(foreground))
        .into_any_element()
}

fn queue_item(id: impl Into<ElementId>, label: String, cx: &App) -> Stateful<Div> {
    let t = tokens(cx);
    h_flex()
        .id(id)
        .role(Role::ListItem)
        .aria_label(label)
        .items_center()
        .justify_between()
        .gap(px(SPACE_2))
        .px(px(SPACE_3))
        .py(px(SPACE_2))
        .bg(t.surface1)
        .border_1()
        .border_color(t.border)
        .rounded(px(RADIUS_LG))
}

/// The queue; `None` when it is empty.
pub fn queue_track(
    items: &[QueuedPrompt],
    held: bool,
    on_action: impl Fn(QueueAction, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Option<AnyElement> {
    if items.is_empty() {
        return None;
    }
    let t = tokens(cx);
    let base = ui_font_size(FONT_BASE, cx);
    let on_action: OnAction = Rc::new(on_action);
    let held_notice = held.then(|| {
        queue_item("composer-queue-held", "Queue paused after Stop".into(), cx)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(base)
                    .text_color(t.foreground)
                    .child("Queue paused after Stop"),
            )
            .child(action_button(
                "composer-queue-resume",
                IconName::Play,
                "Resume queue",
                false,
                QueueAction::Resume,
                &on_action,
                cx,
            ))
    });
    let rows = items.iter().enumerate().map(|(index, item)| {
        let id = &item.id;
        let (up, down) = row_moves(index, items.len());
        let mut actions = Vec::new();
        if up {
            actions.push(action_button(
                ("composer-queued-up", index),
                IconName::ChevronUp,
                "Move queued message up",
                false,
                QueueAction::Move(id.clone(), Direction::Up),
                &on_action,
                cx,
            ));
        }
        if down {
            actions.push(action_button(
                ("composer-queued-down", index),
                IconName::ChevronDown,
                "Move queued message down",
                false,
                QueueAction::Move(id.clone(), Direction::Down),
                &on_action,
                cx,
            ));
        }
        actions.push(action_button(
            ("composer-queued-cancel", index),
            IconName::X,
            "Remove queued message",
            false,
            QueueAction::Cancel(id.clone()),
            &on_action,
            cx,
        ));
        actions.push(action_button(
            ("composer-queued-edit", index),
            IconName::Pencil,
            "Edit queued message",
            false,
            QueueAction::Edit(id.clone()),
            &on_action,
            cx,
        ));
        actions.push(action_button(
            ("composer-queued-send", index),
            IconName::ArrowUp,
            "Send queued message now",
            true,
            QueueAction::SendNow(id.clone()),
            &on_action,
            cx,
        ));
        queue_item(
            ("composer-queued", index),
            format!("Queued: {}", item.text),
            cx,
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .line_clamp(2)
                .text_size(base)
                .text_color(t.foreground)
                .child(item.text.clone()),
        )
        .child(
            h_flex()
                .flex_none()
                .items_center()
                .gap(px(SPACE_2))
                .children(actions),
        )
    });
    Some(
        v_flex()
            .gap(px(SPACE_2))
            .children(held_notice)
            .children(rows)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{Direction, move_target, row_moves};
    use bachuc_client::QueuedPrompt;

    fn items(ids: &[&str]) -> Vec<QueuedPrompt> {
        ids.iter()
            .map(|id| QueuedPrompt {
                id: id.to_string(),
                text: format!("text {id}"),
            })
            .collect()
    }

    #[test]
    fn the_first_row_has_no_move_up_and_the_last_no_move_down() {
        assert_eq!(row_moves(0, 3), (false, true));
        assert_eq!(row_moves(1, 3), (true, true));
        assert_eq!(row_moves(2, 3), (true, false));
        assert_eq!(row_moves(0, 1), (false, false));
    }

    #[test]
    fn up_moves_before_the_previous_item() {
        let items = items(&["a", "b", "c"]);
        assert_eq!(
            move_target(&items, "b", Direction::Up),
            Some(Some("a".into()))
        );
        assert_eq!(move_target(&items, "a", Direction::Up), None);
    }

    #[test]
    fn down_moves_before_the_item_after_next_or_to_the_end() {
        let items = items(&["a", "b", "c"]);
        assert_eq!(
            move_target(&items, "a", Direction::Down),
            Some(Some("c".into()))
        );
        assert_eq!(move_target(&items, "b", Direction::Down), Some(None));
        assert_eq!(move_target(&items, "c", Direction::Down), None);
        assert_eq!(move_target(&items, "x", Direction::Down), None);
    }
}
