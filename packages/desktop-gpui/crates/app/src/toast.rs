//! The toast host: short success and error notes stacked at the bottom right, after Electron's
//! `toast-host.tsx` look (surface, border, 16 px radius, medium shadow). Each note leaves after
//! `TOAST_DURATION`, the time Electron gives an error toast.

use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_BASE, RADIUS_2XL, SPACE_2, SPACE_3, SPACE_4};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::*;
use std::time::Duration;

pub const TOAST_DURATION: Duration = Duration::from_millis(3200);
/// More notes than this drop the oldest, so a burst never covers the window.
pub const MAX_TOASTS: usize = 3;
const TOAST_MAX_WIDTH: f32 = 480.;
const TOAST_ICON: f32 = 18.;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub ok: bool,
    pub message: String,
}

#[derive(Default)]
pub struct ToastStack {
    next: u64,
    items: Vec<Toast>,
}

impl ToastStack {
    /// Adds a note and returns its id. A blank message shows nothing.
    pub fn push(&mut self, ok: bool, message: String) -> Option<u64> {
        let message = message.trim().to_string();
        if message.is_empty() {
            return None;
        }
        self.next += 1;
        self.items.push(Toast {
            id: self.next,
            ok,
            message,
        });
        if self.items.len() > MAX_TOASTS {
            self.items.remove(0);
        }
        Some(self.next)
    }

    /// Removes a note; false when it already left.
    pub fn dismiss(&mut self, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|toast| toast.id != id);
        self.items.len() != before
    }

    pub fn items(&self) -> &[Toast] {
        &self.items
    }
}

/// The notes, oldest on top, or nothing when the stack is empty.
pub fn render_toasts(stack: &ToastStack, cx: &App) -> Option<AnyElement> {
    if stack.items().is_empty() {
        return None;
    }
    let t = tokens(cx);
    let notes = stack.items().iter().map(|toast| {
        let (glyph, color, border) = if toast.ok {
            (IconName::CircleCheck, t.status_success, t.border)
        } else {
            (IconName::TriangleAlert, t.destructive, t.destructive)
        };
        h_flex()
            .id(("toast", toast.id as usize))
            .role(Role::Status)
            .aria_label(toast.message.clone())
            .max_w(px(TOAST_MAX_WIDTH))
            .gap(px(SPACE_2))
            .items_center()
            .py(px(SPACE_2))
            .px(px(SPACE_3))
            .bg(t.surface0)
            .border_1()
            .border_color(border)
            .rounded(px(RADIUS_2XL))
            .shadow(vec![shadow(ShadowSize::Md, cx)])
            .child(
                icon(glyph)
                    .with_size(px(TOAST_ICON))
                    .text_color(color)
                    .flex_shrink_0(),
            )
            .child(
                div()
                    .min_w_0()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .line_height(relative(1.3))
                    .text_color(t.foreground)
                    .child(toast.message.clone()),
            )
    });
    Some(
        v_flex()
            .id("toasts")
            .absolute()
            .bottom(px(SPACE_4))
            .right(px(SPACE_4))
            .items_end()
            .gap(px(SPACE_2))
            .children(notes)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings a `test` macro that shadows the std one.
    use super::{MAX_TOASTS, ToastStack};

    #[test]
    fn toasts_stack_drop_the_oldest_and_leave_by_id() {
        let mut stack = ToastStack::default();
        assert_eq!(stack.push(true, "  ".into()), None);
        let first = stack.push(true, "Committed".into()).expect("shown");
        let second = stack.push(false, " Failed to push ".into()).expect("shown");
        assert_eq!(stack.items()[1].message, "Failed to push");
        assert!(!stack.items()[1].ok);
        for n in 0..MAX_TOASTS {
            stack.push(true, format!("note {n}"));
        }
        assert_eq!(stack.items().len(), MAX_TOASTS);
        assert!(stack.items().iter().all(|toast| toast.id != first));
        assert!(!stack.dismiss(second));
        let last = stack.items()[MAX_TOASTS - 1].id;
        assert!(stack.dismiss(last));
        assert!(!stack.dismiss(last));
        assert_eq!(stack.items().len(), MAX_TOASTS - 1);
    }
}
