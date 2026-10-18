//! The keyboard shortcuts dialog, as `keyboard-shortcuts-dialog.tsx` draws it on desktop: a
//! modal card with a title, a search field, and every row of the shortcut table in its help
//! sections.
//!
//! The rows come from `load_shortcuts`, so an unassigned action shows "Not set" instead of a
//! default that no longer fires. The app shows the view above its content for
//! `shortcuts.dialog.toggle` and removes it after `ShortcutsDialogEvent::Dismissed`: Escape, the
//! close button, or a click outside the card.

use crate::chord::display_combos;
use crate::shortcuts::{Shortcut, ShortcutSection, load_shortcuts, shortcut_section};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_MD, RADIUS_LG, RADIUS_MD, RADIUS_XL, SPACE_0_5, SPACE_1, SPACE_2,
    SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// `settings.shortcuts.dialogTitle` and `settings.shortcuts.searchPlaceholder`.
const TITLE: &str = "Shortcuts";
const PLACEHOLDER: &str = "Search shortcuts";
const UNASSIGNED: &str = "Not set";
const NO_RESULTS: &str = "No results found";
/// The note of the `?` row, `settings.shortcuts.helpNotes.showKeyboardShortcuts`.
const SHOW_SHORTCUTS_NOTE: &str = "Available when focus is not in a text field or terminal.";
/// `desktopCard`: at most 520 px wide and 85% of the window high.
const CARD_MAX_WIDTH: f32 = 520.;
const CARD_MAX_HEIGHT: f32 = 0.85;
/// The desktop overlay is darker than the command center backdrop.
const OVERLAY_OPACITY: f32 = 0.55;
/// One line of FONT_BASE and of FONT_SM text.
const LINE_BASE: f32 = 18.;
const LINE_SM: f32 = 16.;

/// What the dialog tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShortcutsDialogEvent {
    /// Escape, the close button, or a click outside the card.
    Dismissed,
}

/// One help row.
#[derive(Clone, Debug, PartialEq)]
struct HelpRow {
    action: String,
    label: SharedString,
    note: Option<&'static str>,
    /// The display text of each combo, or None when the action is unassigned.
    keys: Option<Vec<String>>,
    /// Lowercased label, note, and every spelling of the chord.
    search: String,
}

#[derive(Clone, Debug, PartialEq)]
struct HelpSection {
    section: ShortcutSection,
    rows: Vec<HelpRow>,
}

fn note_of(action: &str) -> Option<&'static str> {
    (action == "shortcuts.dialog.toggle").then_some(SHOW_SHORTCUTS_NOTE)
}

/// The spellings a search may use for a chord: `cmd+shift+p`, `cmd shift p`, `command+shift+p`,
/// `option`, and the drawn `⇧⌘P`, as `shortcutSearchText`.
fn chord_search_text(chord: &str, keys: &[String]) -> String {
    let lower = chord.to_lowercase();
    let long = lower.replace("cmd", "command").replace("alt", "option");
    let mut parts = vec![lower.replace('+', " "), long.replace('+', " "), lower, long];
    parts.extend(keys.iter().map(|key| key.to_lowercase()));
    parts.join(" ")
}

/// Every row of the table in the Electron section order; a section keeps the table order.
fn help_sections(shortcuts: &[Shortcut]) -> Vec<HelpSection> {
    ShortcutSection::ALL
        .into_iter()
        .filter_map(|section| {
            let rows: Vec<HelpRow> = shortcuts
                .iter()
                .filter(|shortcut| shortcut_section(&shortcut.action) == section)
                .map(|shortcut| {
                    let note = note_of(&shortcut.action);
                    let keys = shortcut
                        .chord
                        .as_deref()
                        .map(display_combos)
                        .filter(|keys| !keys.is_empty());
                    let mut search = vec![shortcut.label.to_lowercase()];
                    search.extend(note.map(str::to_lowercase));
                    if let (Some(chord), Some(keys)) = (shortcut.chord.as_deref(), &keys) {
                        search.push(chord_search_text(chord, keys));
                    }
                    HelpRow {
                        action: shortcut.action.clone(),
                        label: shortcut.label.clone().into(),
                        note,
                        keys,
                        search: search.join(" "),
                    }
                })
                .collect();
            (!rows.is_empty()).then_some(HelpSection { section, rows })
        })
        .collect()
}

/// `filterShortcutHelpSections`: a section whose title matches keeps every row; otherwise a row
/// stays when its search text holds the trimmed query.
fn filter_sections(sections: &[HelpSection], query: &str) -> Vec<HelpSection> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return sections.to_vec();
    }
    sections
        .iter()
        .filter_map(|section| {
            if section.section.title().to_lowercase().contains(&query) {
                return Some(section.clone());
            }
            let rows: Vec<HelpRow> = section
                .rows
                .iter()
                .filter(|row| row.search.contains(&query))
                .cloned()
                .collect();
            (!rows.is_empty()).then_some(HelpSection {
                section: section.section,
                rows,
            })
        })
        .collect()
}

pub struct ShortcutsDialog {
    sections: Vec<HelpSection>,
    visible: Vec<HelpSection>,
    query: Entity<InputState>,
    applied: String,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ShortcutsDialogEvent> for ShortcutsDialog {}

impl ShortcutsDialog {
    pub fn new(window: &mut Window, cx: &mut Context<ShortcutsDialog>) -> ShortcutsDialog {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(PLACEHOLDER));
        let sections = help_sections(&load_shortcuts());
        let weak = cx.weak_entity();
        let subscriptions = vec![
            cx.subscribe(
                &query,
                |this: &mut ShortcutsDialog, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.apply_query(cx);
                    }
                },
            ),
            // The input takes Escape for itself, so the dialog reads it before dispatch.
            cx.intercept_keystrokes(move |event, _, cx| {
                let keystroke = &event.keystroke;
                if keystroke.key != "escape" || keystroke.modifiers.modified() {
                    return;
                }
                if weak
                    .update(cx, |_, cx| cx.emit(ShortcutsDialogEvent::Dismissed))
                    .is_ok()
                {
                    cx.stop_propagation();
                }
            }),
        ];
        query.update(cx, |input, cx| input.focus(window, cx));
        ShortcutsDialog {
            visible: sections.clone(),
            sections,
            query,
            applied: String::new(),
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    fn apply_query(&mut self, cx: &mut Context<Self>) {
        let value = self.query.read(cx).value().to_string();
        if value == self.applied {
            return;
        }
        self.visible = filter_sections(&self.sections, &value);
        self.applied = value;
        cx.notify();
    }

    fn render_row(&self, row: &HelpRow, cx: &App) -> AnyElement {
        let colors = tokens(cx);
        let keys: AnyElement = match &row.keys {
            None => div()
                .flex_shrink_0()
                .text_size(ui_font_size(FONT_BASE, cx))
                .line_height(px(LINE_BASE))
                .text_color(colors.foreground_muted)
                .child(UNASSIGNED)
                .into_any_element(),
            Some(keys) => h_flex()
                .flex_shrink_0()
                .gap(px(SPACE_1))
                .children(keys.iter().map(|text| {
                    div()
                        .px(px(SPACE_1))
                        .py(px(SPACE_0_5))
                        .rounded(px(RADIUS_MD))
                        .bg(colors.surface3.opacity(0.5))
                        .text_size(ui_font_size(FONT_SM, cx))
                        .line_height(px(LINE_SM))
                        .text_color(colors.foreground_muted)
                        .child(text.clone())
                }))
                .into_any_element(),
        };
        h_flex()
            .id(SharedString::from(format!(
                "shortcut-help-row-{}",
                row.action
            )))
            .role(Role::ListItem)
            .aria_label(row.label.clone())
            .justify_between()
            .gap(px(SPACE_3))
            .px(px(SPACE_4))
            .py(px(SPACE_3))
            .border_b_1()
            .border_color(colors.surface2)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .line_height(px(LINE_BASE))
                            .text_color(colors.foreground)
                            .child(row.label.clone()),
                    )
                    .children(row.note.map(|note| {
                        div()
                            .mt(px(SPACE_0_5))
                            .text_size(ui_font_size(FONT_SM, cx))
                            .line_height(px(LINE_SM))
                            .text_color(colors.foreground_muted)
                            .child(note)
                    })),
            )
            .child(keys)
            .into_any_element()
    }
}

impl Render for ShortcutsDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let close = cx.listener(|_, _, _, cx| cx.emit(ShortcutsDialogEvent::Dismissed));
        let view = cx.weak_entity();
        let mut content = div()
            .id("keyboard-shortcuts-dialog-content")
            .flex_shrink(1.)
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .p(px(SPACE_6))
            .flex()
            .flex_col()
            .gap(px(SPACE_4));
        for (position, section) in self.visible.iter().enumerate() {
            let title = section.section.title();
            content = content.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SPACE_2))
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .line_height(px(LINE_SM))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.foreground_muted)
                            .child(title),
                    )
                    .child(
                        div()
                            .id(("shortcut-help-section", position))
                            .role(Role::List)
                            .aria_label(title)
                            .border_1()
                            .border_color(colors.surface2)
                            .rounded(px(RADIUS_LG))
                            .overflow_hidden()
                            .children(section.rows.iter().map(|row| self.render_row(row, cx))),
                    ),
            );
        }
        if self.visible.is_empty() {
            content = content.child(
                div()
                    .id("keyboard-shortcuts-dialog-empty")
                    .role(Role::Status)
                    .aria_label(NO_RESULTS)
                    .py(px(SPACE_6))
                    .text_center()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .line_height(px(LINE_BASE))
                    .text_color(colors.foreground_muted)
                    .child(NO_RESULTS),
            );
        }
        div()
            .id("keyboard-shortcuts-dialog")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p(px(SPACE_6))
            .child(
                div()
                    .id("keyboard-shortcuts-dialog-backdrop")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(black().opacity(OVERLAY_OPACITY))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.emit(ShortcutsDialogEvent::Dismissed)),
                    ),
            )
            .child(
                div()
                    .id("keyboard-shortcuts-dialog-card")
                    .role(Role::Dialog)
                    .aria_label(TITLE)
                    .occlude()
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(px(CARD_MAX_WIDTH))
                    .max_h(relative(CARD_MAX_HEIGHT))
                    .overflow_hidden()
                    .bg(colors.surface1)
                    .rounded(px(RADIUS_XL))
                    .border_1()
                    .border_color(colors.surface2)
                    .text_color(colors.foreground)
                    .child(
                        div()
                            .flex_shrink_0()
                            .border_b_1()
                            .border_color(colors.surface2)
                            .child(
                                h_flex()
                                    .px(px(SPACE_6))
                                    .py(px(SPACE_4))
                                    .gap(px(SPACE_2))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(ui_font_size(FONT_BASE, cx))
                                            .line_height(px(LINE_BASE))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(TITLE),
                                    )
                                    .child(
                                        div()
                                            .id("keyboard-shortcuts-dialog-close")
                                            .role(Role::Button)
                                            .aria_label("Close")
                                            .flex_shrink_0()
                                            .p(px(SPACE_2))
                                            .rounded(px(RADIUS_LG))
                                            .cursor_pointer()
                                            .text_color(colors.foreground_muted)
                                            .active(|style| style.text_color(colors.foreground))
                                            .on_click(close)
                                            .on_a11y_action(
                                                AccessibleAction::Click,
                                                move |_, _, cx| {
                                                    view.update(cx, |_, cx| {
                                                        cx.emit(ShortcutsDialogEvent::Dismissed)
                                                    })
                                                    .ok();
                                                },
                                            )
                                            .child(icon(IconName::X).with_size(px(ICON_MD))),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .px(px(SPACE_6))
                                    .pb(px(SPACE_3))
                                    .gap(px(SPACE_2))
                                    .child(
                                        icon(IconName::Search)
                                            .with_size(px(ICON_MD))
                                            .text_color(colors.foreground_muted),
                                    )
                                    .child(
                                        div().flex_1().min_w_0().child(
                                            Input::new(&self.query)
                                                .appearance(false)
                                                .bordered(false)
                                                .focus_bordered(false)
                                                .px_0()
                                                .py(px(SPACE_2))
                                                // `Input::h` sets only a multi-line height.
                                                .map(|input| {
                                                    Styled::h(input, px(LINE_BASE + SPACE_2 * 2.))
                                                })
                                                .line_height(px(LINE_BASE))
                                                .text_size(ui_font_size(FONT_BASE, cx))
                                                .accessibility_id("keyboard-shortcuts-search")
                                                .aria_label(PLACEHOLDER),
                                        ),
                                    ),
                            ),
                    )
                    .child(content),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{ShortcutSection, filter_sections, help_sections};
    use crate::shortcuts::default_shortcuts;

    #[test]
    fn every_row_shows_once_in_its_section_order() {
        let shortcuts = default_shortcuts();
        let sections = help_sections(&shortcuts);
        let order: Vec<ShortcutSection> = sections.iter().map(|s| s.section).collect();
        assert_eq!(order, ShortcutSection::ALL);
        let rows: usize = sections.iter().map(|s| s.rows.len()).sum();
        assert_eq!(rows, shortcuts.len());
        let general: Vec<&str> = sections[0].rows.iter().map(|r| r.action.as_str()).collect();
        assert_eq!(
            general[..3],
            [
                "command-center.toggle",
                "command-center.files",
                "shortcuts.dialog.toggle"
            ]
        );
        let help = &sections[0].rows[2];
        assert_eq!(help.keys.as_deref(), Some(&["?".to_string()][..]));
        assert!(help.note.is_some());
    }

    #[test]
    fn an_unassigned_row_has_no_keys() {
        let mut shortcuts = default_shortcuts();
        shortcuts[0].chord = None;
        let sections = help_sections(&shortcuts);
        assert_eq!(sections[0].rows[0].keys, None);
    }

    #[test]
    fn search_matches_label_keys_and_section_title() {
        let sections = help_sections(&default_shortcuts());
        let actions = |query: &str| -> Vec<String> {
            filter_sections(&sections, query)
                .into_iter()
                .flat_map(|s| s.rows.into_iter().map(|r| r.action))
                .collect()
        };
        assert_eq!(actions("pin chat"), ["workspace.pin"]);
        assert!(actions("cmd k").contains(&"command-center.toggle".to_string()));
        assert!(actions("command+shift+p").contains(&"workspace.pin".to_string()));
        assert!(actions("⌘K").contains(&"command-center.toggle".to_string()));
        assert_eq!(actions("layout").len(), 3);
        assert!(actions("  ").len() == default_shortcuts().len());
        assert!(filter_sections(&sections, "zzz nothing").is_empty());
    }
}
