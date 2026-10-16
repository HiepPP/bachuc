//! The Shortcuts pane: list, rebind by recording a chord, clear, and reset. Chords are stored
//! with `bachuc_command::save_overrides`.

use crate::bindings::{
    Section, combo_from_keystroke, conflict, is_overridden, reset, reset_all, section_of, set_chord,
};
use crate::ui::{card, column, error_text, row, scroll_page, section};
use bachuc_command::{Shortcut, display_combos, load_shortcuts_from, save_overrides_to};
use bachuc_icons::IconName;
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, RADIUS_MD, SPACE_0_5, SPACE_1, SPACE_2};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::path::PathBuf;

/// The stored shortcuts changed. The app rebinds its keys.
pub struct ShortcutsChanged;

struct Capture {
    action: String,
    combos: Vec<String>,
}

pub struct ShortcutsPane {
    shortcuts: Vec<Shortcut>,
    capture: Option<Capture>,
    error: Option<SharedString>,
    dir: PathBuf,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ShortcutsChanged> for ShortcutsPane {}

impl ShortcutsPane {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self::in_dir(bachuc_command::data_dir(), cx)
    }

    /// `new` with the folder of `shortcuts.json` given.
    pub fn in_dir(dir: PathBuf, cx: &mut Context<Self>) -> Self {
        let weak = cx.weak_entity();
        let subscriptions = vec![
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
            // Recording reads every key press before the app's own bindings see it.
            cx.intercept_keystrokes(move |event, _, cx| {
                let stroke = event.keystroke.clone();
                let handled = weak
                    .update(cx, |this, cx| this.record(&stroke, cx))
                    .unwrap_or(false);
                if handled {
                    cx.stop_propagation();
                }
            }),
        ];
        Self {
            shortcuts: load_shortcuts_from(&dir),
            capture: None,
            error: None,
            dir,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    /// Takes one key press while a row records. Backspace alone removes the last combo.
    fn record(&mut self, stroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        let Some(capture) = &mut self.capture else {
            return false;
        };
        let modifiers = &stroke.modifiers;
        let bare = !(modifiers.platform || modifiers.control || modifiers.alt || modifiers.shift);
        if bare && stroke.key == "backspace" {
            capture.combos.pop();
        } else if let Some(combo) = combo_from_keystroke(stroke) {
            capture.combos.push(combo);
        } else {
            return false;
        }
        cx.notify();
        true
    }

    fn start(&mut self, action: &str, cx: &mut Context<Self>) {
        self.error = None;
        self.capture = Some(Capture {
            action: action.to_string(),
            combos: Vec::new(),
        });
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.capture = None;
        cx.notify();
    }

    /// The recorded chord and the action that already uses it.
    fn pending(&self) -> Option<(String, Option<&Shortcut>)> {
        let capture = self.capture.as_ref()?;
        if capture.combos.is_empty() {
            return None;
        }
        let chord = capture.combos.join(" ");
        let other = conflict(&self.shortcuts, &capture.action, &chord);
        Some((chord, other))
    }

    fn done(&mut self, cx: &mut Context<Self>) {
        let Some((chord, other)) = self.pending() else {
            return;
        };
        if other.is_some() {
            return;
        }
        let action = self.capture.take().map(|c| c.action).unwrap_or_default();
        self.change(cx, |shortcuts| set_chord(shortcuts, &action, Some(chord)));
    }

    fn clear(&mut self, action: &str, cx: &mut Context<Self>) {
        self.capture = None;
        self.change(cx, |shortcuts| set_chord(shortcuts, action, None));
    }

    fn reset(&mut self, action: &str, cx: &mut Context<Self>) {
        self.capture = None;
        self.change(cx, |shortcuts| reset(shortcuts, action));
    }

    fn reset_all(&mut self, cx: &mut Context<Self>) {
        self.capture = None;
        self.change(cx, |shortcuts| {
            reset_all(shortcuts);
            true
        });
    }

    /// Applies `edit`, stores the overrides, and tells the app. A failed save keeps the old
    /// table.
    fn change(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut Vec<Shortcut>) -> bool) {
        let mut next = self.shortcuts.clone();
        if edit(&mut next) {
            match save_overrides_to(&self.dir, &next) {
                Ok(()) => {
                    self.shortcuts = next;
                    self.error = None;
                    cx.emit(ShortcutsChanged);
                }
                Err(error) => {
                    log::warn!("cannot save shortcuts: {error:#}");
                    self.error = Some(format!("Could not save the shortcuts: {error}").into());
                }
            }
        }
        cx.notify();
    }

    fn keys(&self, chord: &str, cx: &App) -> Div {
        let colors = tokens(cx);
        h_flex()
            .gap(px(SPACE_1))
            .children(display_combos(chord).into_iter().map(|text| {
                div()
                    .px(px(SPACE_1))
                    .py(px(SPACE_0_5))
                    .rounded(px(RADIUS_MD))
                    .bg(colors.surface3.opacity(0.5))
                    .text_size(ui_font_size(FONT_SM, cx))
                    .text_color(colors.foreground_muted)
                    .child(text)
            }))
    }

    fn shortcut_row(
        &self,
        index: usize,
        shortcut: &Shortcut,
        first: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let action = shortcut.action.clone();
        let capturing = self
            .capture
            .as_ref()
            .filter(|capture| capture.action == shortcut.action);
        let overridden = is_overridden(&self.shortcuts, &shortcut.action);
        let label = shortcut.label.clone();

        let keys = if let Some(capture) = capturing {
            if capture.combos.is_empty() {
                div()
                    .text_color(colors.foreground_muted)
                    .child("Press shortcut...")
                    .into_any_element()
            } else {
                self.keys(&capture.combos.join(" "), cx).into_any_element()
            }
        } else if let Some(chord) = &shortcut.chord {
            self.keys(chord, cx).into_any_element()
        } else {
            div()
                .text_color(colors.foreground_muted)
                .child("Not set")
                .into_any_element()
        };

        let clash = capturing
            .and_then(|_| self.pending())
            .and_then(|(_, other)| other.map(|other| format!("Already used by {}", other.label)));

        let actions: AnyElement = if capturing.is_some() {
            // Electron shows Done once a combo is recorded. It has no conflict state, so a clash
            // here disables Done and names the other action.
            let recorded = self.pending().is_some();
            h_flex()
                .gap(px(SPACE_2))
                .when(recorded, |this| {
                    this.child(
                        Button::new(("shortcut-done", index))
                            .ghost()
                            .small()
                            .label("Done")
                            .disabled(clash.is_some())
                            .accessibility_label(format!("Save shortcut for {label}"))
                            .on_click(cx.listener(|this, _, _, cx| this.done(cx))),
                    )
                })
                .child(
                    Button::new(("shortcut-cancel", index))
                        .ghost()
                        .small()
                        .label("Cancel")
                        .accessibility_label(format!("Cancel recording for {label}"))
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                )
                .into_any_element()
        } else {
            let bind = {
                let action = action.clone();
                crate::ui::icon_button(
                    ("shortcut-bind", index),
                    IconName::Pencil,
                    if shortcut.chord.is_some() {
                        format!("Rebind {label}")
                    } else {
                        format!("Bind {label}")
                    },
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.start(&action, cx)))
            };
            let clear = {
                let action = action.clone();
                crate::ui::icon_button(
                    ("shortcut-clear", index),
                    IconName::X,
                    format!("Clear {label}"),
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.clear(&action, cx)))
            };
            let restore = {
                let action = action.clone();
                crate::ui::icon_button(
                    ("shortcut-reset", index),
                    IconName::Undo2,
                    format!("Reset {label} to default"),
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.reset(&action, cx)))
            };
            h_flex()
                .gap(px(SPACE_1))
                .child(bind)
                .when(shortcut.chord.is_some(), |this| this.child(clear))
                .when(overridden, |this| this.child(restore))
                .into_any_element()
        };

        v_flex()
            .id(("shortcut-row", index))
            .role(Role::ListItem)
            .aria_label(label.clone())
            .child(
                row(first, cx)
                    .when(capturing.is_some(), |this| this.bg(colors.surface2))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .text_color(colors.foreground)
                            .child(label),
                    )
                    .child(
                        v_flex()
                            .items_end()
                            .gap(px(SPACE_1))
                            .child(keys)
                            .children(clash.map(|text| error_text(text, cx))),
                    )
                    .child(actions),
            )
            .into_any_element()
    }
}

impl Render for ShortcutsPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let any_override = self
            .shortcuts
            .iter()
            .any(|s| is_overridden(&self.shortcuts, &s.action));
        let shortcuts = self.shortcuts.clone();
        let mut body = column().children(self.error.clone().map(|error| error_text(error, cx)));
        for (position, group) in Section::ALL.into_iter().enumerate() {
            let rows: Vec<AnyElement> = shortcuts
                .iter()
                .enumerate()
                .filter(|(_, s)| section_of(&s.action) == group)
                .enumerate()
                .map(|(first, (index, shortcut))| {
                    self.shortcut_row(index, shortcut, first == 0, cx)
                })
                .collect();
            let trailing = (position == 0 && any_override).then(|| {
                Button::new("shortcuts-reset-all")
                    .ghost()
                    .small()
                    .label("Reset all")
                    .accessibility_label("Reset all shortcuts")
                    .on_click(cx.listener(|this, _, _, cx| this.reset_all(cx)))
                    .into_any_element()
            });
            body = body.child(section(
                group.title(),
                trailing,
                card(cx)
                    .id(("shortcut-section", position))
                    .role(Role::List)
                    .aria_label(group.title())
                    .children(rows),
                cx,
            ));
        }
        scroll_page("shortcuts-pane", &self.scroll, body, cx)
    }
}
