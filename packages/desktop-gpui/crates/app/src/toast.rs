//! The toast host: one short success or error note at the top center, below the header, after
//! Electron's `toast-host.tsx` (surface, border, 16 px radius, medium shadow). Each note carries
//! its own duration from `toast_duration`.

use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_BASE, HEADER_HEIGHT, RADIUS_2XL, SPACE_2, SPACE_3, SPACE_4};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size, zoomed};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::*;
use std::time::Duration;

/// Electron's `DEFAULT_DURATION_MS`, for a success note.
const SUCCESS_DURATION: Duration = Duration::from_millis(2200);
/// Electron's `toast.error` duration.
const ERROR_DURATION: Duration = Duration::from_millis(3200);
/// Electron's host holds one toast; a new note replaces the shown one.
pub const MAX_TOASTS: usize = 1;
const TOAST_MAX_WIDTH: f32 = 480.;
const TOAST_ICON: f32 = 18.;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub ok: bool,
    pub message: String,
    pub duration: Duration,
}

/// How long a note shows, as Electron's `toast.show` (2.2 s) and `toast.error` (3.2 s).
pub fn toast_duration(ok: bool) -> Duration {
    if ok { SUCCESS_DURATION } else { ERROR_DURATION }
}

#[derive(Default)]
pub struct ToastStack {
    next: u64,
    items: Vec<Toast>,
}

impl ToastStack {
    /// Adds a note and returns it. A blank message shows nothing.
    pub fn push(&mut self, ok: bool, message: String) -> Option<&Toast> {
        let message = message.trim().to_string();
        if message.is_empty() {
            return None;
        }
        self.next += 1;
        self.items.push(Toast {
            id: self.next,
            ok,
            message,
            duration: toast_duration(ok),
        });
        if self.items.len() > MAX_TOASTS {
            self.items.remove(0);
        }
        self.items.last()
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
            .max_w(zoomed(TOAST_MAX_WIDTH, cx))
            .gap(zoomed(SPACE_2, cx))
            .items_center()
            .py(zoomed(SPACE_2, cx))
            .px(zoomed(SPACE_3, cx))
            .bg(t.surface0)
            .border_1()
            .border_color(border)
            .rounded(zoomed(RADIUS_2XL, cx))
            .shadow(vec![shadow(ShadowSize::Md, cx)])
            .child(
                icon(glyph)
                    .with_size(zoomed(TOAST_ICON, cx))
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
            .top(zoomed(HEADER_HEIGHT + SPACE_2, cx))
            .left(zoomed(SPACE_4, cx))
            .right(zoomed(SPACE_4, cx))
            .items_center()
            .child(
                v_flex()
                    .w(relative(0.92))
                    .max_w(zoomed(TOAST_MAX_WIDTH, cx))
                    .items_center()
                    .gap(zoomed(SPACE_2, cx))
                    .children(notes),
            )
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings a `test` macro that shadows the std one.
    use super::{MAX_TOASTS, ToastStack, toast_duration};
    use std::time::Duration;

    #[test]
    fn a_new_toast_replaces_the_shown_one_and_leaves_by_id() {
        let mut stack = ToastStack::default();
        assert!(stack.push(true, "  ".into()).is_none());
        let first = stack.push(true, "Committed".into()).expect("shown").id;
        let second = stack
            .push(false, " Failed to push ".into())
            .expect("shown")
            .clone();
        assert_eq!(second.message, "Failed to push");
        assert!(!second.ok);
        assert_eq!(stack.items().len(), MAX_TOASTS);
        assert!(!stack.dismiss(first));
        assert!(stack.dismiss(second.id));
        assert!(!stack.dismiss(second.id));
        assert!(stack.items().is_empty());
    }

    #[test]
    fn a_success_toast_lasts_2_2_s_and_an_error_toast_3_2_s() {
        assert_eq!(toast_duration(true), Duration::from_millis(2200));
        assert_eq!(toast_duration(false), Duration::from_millis(3200));
        let mut stack = ToastStack::default();
        let ok = stack
            .push(true, "Committed".into())
            .expect("shown")
            .duration;
        let error = stack.push(false, "Failed".into()).expect("shown").duration;
        assert_eq!((ok, error), (toast_duration(true), toast_duration(false)));
    }
}
