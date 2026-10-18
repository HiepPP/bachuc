//! The Projects pane: the list of projects, and for one project the editor of its `paseo.json`
//! (worktree hooks, scripts, and metadata prompts), after `project-settings-screen.tsx`.

use crate::config::{
    ConfigDraft, PROMPT_PLACEHOLDERS, PROMPT_TITLES, ScriptDraft, apply_draft, config_to_draft,
    script_errors, script_hint,
};
use crate::ui::{
    callout, card, column, error_text, field_label, icon_button, note, page_title, row, row_text,
    scroll_page, section,
};
use bachuc_client::{Client, ClientEvent};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_SM, SPACE_1, SPACE_2, SPACE_3, SPACE_4, WEIGHT_MEDIUM,
};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::Value;

/// The editor asks to return to the project list.
pub struct Back;

enum Load {
    Loading,
    Failed(SharedString),
    Ready,
}

enum WriteError {
    /// The file changed after the last read; the user reloads before saving.
    Stale,
    Failed(SharedString),
}

struct ScriptEdit {
    id: usize,
    /// A script added by the Add button and not applied yet.
    fresh: bool,
    name: Entity<InputState>,
    command: Entity<TextareaState>,
    service: bool,
    touched: bool,
}

pub struct ProjectEditor {
    client: Entity<Client>,
    root: String,
    name: String,
    load: Load,
    /// The config as last read or written; saving keeps its keys the form does not edit.
    base: Value,
    draft: ConfigDraft,
    setup: Entity<TextareaState>,
    teardown: Entity<TextareaState>,
    prompts: Vec<Entity<TextareaState>>,
    editing: Option<ScriptEdit>,
    /// The id of the script that waits for a remove confirmation.
    removing: Option<usize>,
    saving: bool,
    saved: bool,
    write_error: Option<WriteError>,
    scroll: ScrollHandle,
    task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Back> for ProjectEditor {}

fn textarea(
    placeholder: &str,
    window: &mut Window,
    cx: &mut Context<ProjectEditor>,
) -> Entity<TextareaState> {
    let placeholder = placeholder.to_string();
    cx.new(|cx| {
        TextareaState::new(window, cx)
            .placeholder(placeholder)
            .auto_grow(3, 10)
    })
}

fn set_text(
    state: &Entity<TextareaState>,
    text: &str,
    window: &mut Window,
    cx: &mut Context<ProjectEditor>,
) {
    state.update(cx, |state, cx| {
        state.set_value(text.to_string(), window, cx)
    });
}

impl ProjectEditor {
    pub fn new(
        client: Entity<Client>,
        root: String,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let setup = textarea("npm install", window, cx);
        let teardown = textarea("docker compose down", window, cx);
        let prompts = PROMPT_PLACEHOLDERS
            .iter()
            .map(|placeholder| textarea(placeholder, window, cx))
            .collect();
        let mut editor = Self {
            client,
            root,
            name,
            load: Load::Loading,
            base: Value::Null,
            draft: ConfigDraft::empty(),
            setup,
            teardown,
            prompts,
            editing: None,
            removing: None,
            saving: false,
            saved: false,
            write_error: None,
            scroll: ScrollHandle::new(),
            task: None,
            _subscriptions: Vec::new(),
        };
        editor._subscriptions = vec![
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
            // A request sent before `Connected` fails and is not retried. A reconnect reads
            // the file again unless a loaded draft is on screen; a stale write still reports.
            cx.subscribe_in(
                &editor.client,
                window,
                |this: &mut ProjectEditor, _, event: &ClientEvent, window, cx| {
                    if matches!(event, ClientEvent::Connected) && !matches!(this.load, Load::Ready)
                    {
                        this.reload(window, cx);
                    }
                },
            ),
        ];
        editor.reload(window, cx);
        editor
    }

    /// Reads `paseo.json` again and discards the draft. Before `Connected`, it only shows the
    /// loading state; the `Connected` subscription reads it.
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.load = Load::Loading;
        self.write_error = None;
        self.saved = false;
        self.editing = None;
        self.removing = None;
        if !self.client.read(cx).is_connected() {
            self.task = None;
            cx.notify();
            return;
        }
        let root = self.root.clone();
        let read = self
            .client
            .update(cx, |client, cx| client.read_project_config(&root, cx));
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = read.await;
            let _ = this.update_in(cx, |this, window, cx| this.loaded(result, window, cx));
        }));
        cx.notify();
    }

    fn loaded(
        &mut self,
        result: anyhow::Result<Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(config) => {
                self.draft = config_to_draft(&config);
                self.base = config;
                set_text(&self.setup, &self.draft.setup_text, window, cx);
                set_text(&self.teardown, &self.draft.teardown_text, window, cx);
                for (state, text) in self.prompts.iter().zip(&self.draft.prompts) {
                    set_text(state, text, window, cx);
                }
                self.load = Load::Ready;
            }
            Err(error) => self.load = Load::Failed(error.to_string().into()),
        }
        cx.notify();
    }

    /// Copies the text fields into the draft.
    fn collect(&mut self, cx: &App) {
        self.draft.setup_text = self.setup.read(cx).value().to_string();
        self.draft.teardown_text = self.teardown.read(cx).value().to_string();
        for (slot, state) in self.draft.prompts.iter_mut().zip(&self.prompts) {
            *slot = state.read(cx).value().to_string();
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if self.saving || matches!(self.write_error, Some(WriteError::Stale)) {
            return;
        }
        if self.draft.scripts.iter().any(|s| {
            let (name, command) = script_errors(s);
            name.is_some() || command.is_some()
        }) {
            return;
        }
        self.collect(cx);
        let config = apply_draft(&self.draft, &self.base);
        let root = self.root.clone();
        let sent = config.clone();
        let write = self.client.update(cx, |client, cx| {
            client.write_project_config(&root, sent, cx)
        });
        self.saving = true;
        self.saved = false;
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = write.await;
            let _ = this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(()) => {
                        this.base = config;
                        this.write_error = None;
                        this.saved = true;
                    }
                    Err(error) if error.to_string() == "stale_project_config" => {
                        this.write_error = Some(WriteError::Stale);
                    }
                    Err(error) => {
                        this.write_error = Some(WriteError::Failed(error.to_string().into()));
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn start_edit(&mut self, id: usize, fresh: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(script) = self.draft.scripts.iter().find(|s| s.id == id).cloned() else {
            return;
        };
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("dev")
                .default_value(script.name.clone())
        });
        let command = textarea("npm run dev", window, cx);
        set_text(&command, &script.command_text, window, cx);
        name.update(cx, |input, cx| input.focus(window, cx));
        self.removing = None;
        self.editing = Some(ScriptEdit {
            id,
            fresh,
            name,
            command,
            service: script.kind == "service",
            touched: false,
        });
        cx.notify();
    }

    fn add_script(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let script = self.draft.new_script();
        let id = script.id;
        self.draft.scripts.push(script);
        self.start_edit(id, true, window, cx);
    }

    /// The script being edited with the form values applied.
    fn edited(&self, cx: &App) -> Option<ScriptDraft> {
        let edit = self.editing.as_ref()?;
        let mut script = self.draft.scripts.iter().find(|s| s.id == edit.id)?.clone();
        script.name = edit.name.read(cx).value().to_string();
        script.command_text = edit.command.read(cx).value().to_string();
        script.kind = if edit.service { "service" } else { "" }.to_string();
        Some(script)
    }

    fn apply_edit(&mut self, cx: &mut Context<Self>) {
        let Some(script) = self.edited(cx) else {
            return;
        };
        let (name_error, command_error) = script_errors(&script);
        if name_error.is_some() || command_error.is_some() {
            if let Some(edit) = &mut self.editing {
                edit.touched = true;
            }
            cx.notify();
            return;
        }
        if let Some(slot) = self.draft.scripts.iter_mut().find(|s| s.id == script.id) {
            *slot = script;
        }
        self.editing = None;
        cx.notify();
    }

    fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        if let Some(script) = self.edited(cx) {
            let blank = script.name.trim().is_empty()
                && script.command_text.trim().is_empty()
                && script.kind.is_empty()
                && script.port_text.trim().is_empty();
            let fresh = self.editing.as_ref().is_some_and(|e| e.fresh);
            if blank && fresh {
                self.draft.scripts.retain(|s| s.id != script.id);
            }
        }
        self.editing = None;
        cx.notify();
    }

    fn remove_script(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.removing.take() {
            self.draft.scripts.retain(|s| s.id != id);
        }
        cx.notify();
    }

    fn callout_buttons(&self, cx: &mut Context<Self>, retry: bool) -> AnyElement {
        h_flex()
            .gap(px(SPACE_2))
            .when(retry, |this| {
                this.child(
                    Button::new("project-try-again")
                        .outline()
                        .small()
                        .label("Try again")
                        .accessibility_label("Try again")
                        .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                )
            })
            .child(
                Button::new("project-reload")
                    .outline()
                    .small()
                    .label("Reload")
                    .accessibility_label("Reload")
                    .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
            )
            .into_any_element()
    }

    fn script_row(
        &self,
        position: usize,
        script: &ScriptDraft,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = script.id;
        let title = if script.name.is_empty() {
            "Untitled script".to_string()
        } else {
            script.name.clone()
        };
        let hint = script_hint(script);
        let removing = self.removing == Some(id);
        let mut item = v_flex()
            .id(("script-row", id))
            .role(Role::ListItem)
            .aria_label(title.clone())
            .child(
                row(position == 0, cx)
                    .child(
                        div()
                            .id(("script-edit-area", id))
                            .role(Role::Button)
                            .aria_label(format!("Edit {title}"))
                            .flex_1()
                            .min_w_0()
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.start_edit(id, false, window, cx)
                            }))
                            .child(row_text(
                                title.clone(),
                                (!hint.is_empty()).then(|| hint.into()),
                                cx,
                            )),
                    )
                    .child(
                        icon_button(
                            ("script-edit", id),
                            IconName::Pencil,
                            format!("Edit {title}"),
                            cx,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| this.start_edit(id, false, window, cx),
                        )),
                    )
                    .child(
                        icon_button(
                            ("script-remove", id),
                            IconName::Trash2,
                            format!("Remove {title}"),
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.removing = Some(id);
                            cx.notify();
                        })),
                    ),
            );
        if removing {
            item = item.child(
                div().px(px(SPACE_4)).pb(px(SPACE_4)).child(
                    callout(cx)
                        .id(("script-remove-confirm", id))
                        .role(Role::AlertDialog)
                        .aria_label("Remove script?")
                        .child(div().child(format!("Remove {title}?")))
                        .child(
                            h_flex()
                                .gap(px(SPACE_2))
                                .justify_end()
                                .child(
                                    Button::new("script-remove-cancel")
                                        .ghost()
                                        .small()
                                        .label("Cancel")
                                        .accessibility_label("Cancel remove script")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.removing = None;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("script-remove-confirm-button")
                                        .danger()
                                        .small()
                                        .label("Remove")
                                        .accessibility_label("Confirm remove script")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.remove_script(cx)),
                                        ),
                                ),
                        ),
                ),
            );
        }
        item.into_any_element()
    }

    fn script_form(&self, edit: &ScriptEdit, cx: &mut Context<Self>) -> AnyElement {
        let script = self.edited(cx);
        let (name_error, command_error) = script.as_ref().map_or((None, None), script_errors);
        let title = script
            .as_ref()
            .filter(|s| !s.name.trim().is_empty())
            .map_or("New script".to_string(), |s| format!("Edit {}", s.name));
        let colors = tokens(cx);
        let service = edit.service;
        card(cx)
            .id("script-form")
            .role(Role::Form)
            .aria_label(title.clone())
            .mb(px(SPACE_3))
            .p(px(SPACE_4))
            .gap(px(SPACE_3))
            .child(div().font_weight(FontWeight(WEIGHT_MEDIUM)).child(title))
            .child(
                v_flex()
                    .gap(px(SPACE_1))
                    .child(field_label("Name", cx))
                    .child(
                        Input::new(&edit.name)
                            .accessibility_id("script-name-input")
                            .aria_label("Script name"),
                    )
                    .children(
                        name_error
                            .filter(|_| edit.touched)
                            .map(|text| error_text(text, cx)),
                    ),
            )
            .child(
                v_flex()
                    .gap(px(SPACE_1))
                    .child(field_label("Command", cx))
                    .child(
                        Textarea::new(&edit.command)
                            .accessibility_id("script-command-input")
                            .aria_label("Script command"),
                    )
                    .children(
                        command_error
                            .filter(|_| edit.touched)
                            .map(|text| error_text(text, cx)),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(SPACE_3))
                    .items_center()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(SPACE_1))
                            .child(div().child("Run as a service"))
                            .child(
                                div()
                                    .text_size(ui_font_size(FONT_SM, cx))
                                    .text_color(colors.foreground_muted)
                                    .child("Paseo supervises the process and assigns a port via $PASEO_PORT"),
                            ),
                    )
                    .child(
                        Switch::new("script-service-switch")
                            .checked(service)
                            .accessibility_label("Run as a service")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                if let Some(edit) = &mut this.editing {
                                    edit.service = *checked;
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(SPACE_2))
                    .justify_end()
                    .child(
                        Button::new("script-form-cancel")
                            .ghost()
                            .label("Cancel")
                            .accessibility_label("Cancel script")
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_edit(cx))),
                    )
                    .child(
                        Button::new("script-form-save")
                            .primary()
                            .label("Save")
                            .accessibility_label("Save script")
                            .on_click(cx.listener(|this, _, _, cx| this.apply_edit(cx))),
                    ),
            )
            .into_any_element()
    }

    fn render_form(&mut self, cx: &mut Context<Self>) -> Div {
        let colors = tokens(cx);
        let scripts: Vec<ScriptDraft> = self.draft.scripts.clone();
        let editing_id = self.editing.as_ref().map(|e| e.id);
        let rows: Vec<AnyElement> = scripts
            .iter()
            .filter(|s| Some(s.id) != editing_id)
            .enumerate()
            .map(|(position, script)| self.script_row(position, script, cx))
            .collect();
        let script_list = if rows.is_empty() && self.editing.is_none() {
            card(cx)
                .child(row(true, cx).child(note("No scripts yet.", cx)))
                .into_any_element()
        } else if rows.is_empty() {
            div().into_any_element()
        } else {
            card(cx)
                .id("script-list")
                .role(Role::List)
                .aria_label("Scripts")
                .children(rows)
                .into_any_element()
        };
        let add = Button::new("script-add")
            .ghost()
            .small()
            .icon(icon(IconName::Plus))
            .label("Add script")
            .accessibility_label("Add script")
            .on_click(cx.listener(|this, _, window, cx| this.add_script(window, cx)))
            .into_any_element();
        let script_form = self.editing.as_ref().map(|edit| self.script_form(edit, cx));

        let prompt_sections: Vec<Div> = self
            .prompts
            .iter()
            .enumerate()
            .map(|(index, state)| {
                v_flex()
                    .gap(px(SPACE_2))
                    .mb(px(SPACE_3))
                    .child(field_label(PROMPT_TITLES[index], cx))
                    .child(
                        Textarea::new(state)
                            .accessibility_id(format!("metadata-prompt-{index}"))
                            .aria_label(PROMPT_TITLES[index]),
                    )
            })
            .collect();

        let stale = matches!(self.write_error, Some(WriteError::Stale));
        let write_callout = match &self.write_error {
            Some(WriteError::Stale) => Some(
                callout(cx)
                    .id("project-stale")
                    .role(Role::Alert)
                    .aria_label("Config changed on disk")
                    .child(div().child("Config changed on disk"))
                    .child(note(
                        "Reload to fetch the latest paseo.json before saving.",
                        cx,
                    ))
                    .child(self.callout_buttons(cx, false)),
            ),
            Some(WriteError::Failed(detail)) => Some(
                callout(cx)
                    .id("project-write-failed")
                    .role(Role::Alert)
                    .aria_label("Couldn't save paseo.json")
                    .child(div().child("Couldn't save paseo.json"))
                    .child(note(
                        format!("Try again, or reload the latest version from disk. ({detail})"),
                        cx,
                    ))
                    .child(self.callout_buttons(cx, true)),
            ),
            None => None,
        };

        v_flex()
            .w_full()
            .child(section(
                "Worktree lifecycle hooks",
                None,
                v_flex()
                    .child(
                        v_flex()
                            .gap(px(SPACE_2))
                            .mb(px(SPACE_3))
                            .child(field_label("Setup", cx))
                            .child(
                                Textarea::new(&self.setup)
                                    .accessibility_id("worktree-setup-input")
                                    .aria_label("Worktree setup commands"),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap(px(SPACE_2))
                            .child(field_label("Teardown", cx))
                            .child(
                                Textarea::new(&self.teardown)
                                    .accessibility_id("worktree-teardown-input")
                                    .aria_label("Worktree teardown commands"),
                            ),
                    ),
                cx,
            ))
            .child(section(
                "Scripts",
                Some(add),
                v_flex().children(script_form).child(script_list),
                cx,
            ))
            .child(section(
                "Metadata generation",
                None,
                v_flex().children(prompt_sections),
                cx,
            ))
            .children(write_callout.map(|callout| div().mb(px(SPACE_3)).child(callout)))
            .child(
                h_flex()
                    .items_center()
                    .justify_end()
                    .gap(px(SPACE_3))
                    .children(self.saved.then(|| {
                        div()
                            .id("project-saved")
                            .role(Role::Status)
                            .aria_label("Project saved")
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.status_success)
                            .child("Project saved")
                    }))
                    .child(
                        Button::new("project-save")
                            .primary()
                            .label(if self.saving { "Saving..." } else { "Save" })
                            .accessibility_label("Save")
                            .disabled(
                                self.saving
                                    || stale
                                    || self.draft.scripts.iter().any(|s| {
                                        let (name, command) = script_errors(s);
                                        name.is_some() || command.is_some()
                                    }),
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                    ),
            )
    }
}

fn failure_copy(detail: &str) -> (&'static str, String) {
    match detail {
        "invalid_project_config" => (
            "paseo.json couldn't be parsed",
            "Fix the file on disk, then reload.".into(),
        ),
        "project_not_found" => (
            "This host doesn't have this project",
            "The selected host has no record of this project.".into(),
        ),
        _ => ("Couldn't load paseo.json", detail.to_string()),
    }
}

impl Render for ProjectEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let header = h_flex()
            .items_center()
            .gap(px(SPACE_3))
            .mb(px(SPACE_4))
            .child(
                Button::new("project-back")
                    .ghost()
                    .small()
                    .icon(icon(IconName::ArrowLeft))
                    .label("Back to projects")
                    .accessibility_label("Back to projects")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Back))),
            )
            .child(
                div()
                    .id("project-name")
                    .role(Role::Heading)
                    .aria_label(self.name.clone())
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .font_weight(FontWeight(WEIGHT_MEDIUM))
                    .child(self.name.clone()),
            );
        let page = column().child(page_title("Projects", cx)).child(header);
        let body = match &self.load {
            Load::Loading => page.child(
                div()
                    .id("project-loading")
                    .role(Role::Status)
                    .aria_label("Loading project config")
                    .text_color(colors.foreground_muted)
                    .child("Loading..."),
            ),
            Load::Failed(detail) => {
                let (title, text) = failure_copy(detail);
                page.child(
                    callout(cx)
                        .id("project-read-failed")
                        .role(Role::Alert)
                        .aria_label(title)
                        .child(div().child(title))
                        .child(note(text, cx))
                        .child(self.callout_buttons(cx, false)),
                )
            }
            Load::Ready => page.child(self.render_form(cx)),
        };
        scroll_page("project-editor", &self.scroll, body, cx)
    }
}

/// The pane: the project list, then the editor of the picked project.
pub struct ProjectsPane {
    client: Entity<Client>,
    editor: Option<Entity<ProjectEditor>>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl ProjectsPane {
    pub fn new(client: Entity<Client>, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.observe(&client, |_, _, cx| cx.notify()),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        Self {
            client,
            editor: None,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    /// Opens the editor of the project with this root folder.
    pub fn open(&mut self, root: &str, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let (root, name) = (root.to_string(), name.to_string());
        let editor = cx.new(|cx| ProjectEditor::new(client, root, name, window, cx));
        self._subscriptions.push(cx.subscribe(
            &editor,
            |this: &mut ProjectsPane, _, _: &Back, cx| {
                this.editor = None;
                cx.notify();
            },
        ));
        self.editor = Some(editor);
        cx.notify();
    }
}

impl Render for ProjectsPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(editor) = &self.editor {
            return div().size_full().child(editor.clone()).into_any_element();
        }
        let colors = tokens(cx);
        let projects = self.client.read(cx).projects().to_vec();
        let rows: Vec<AnyElement> = projects
            .iter()
            .enumerate()
            .map(|(index, project)| {
                let (root, name) = (project.root.clone(), project.name.clone());
                row(index == 0, cx)
                    .id(("project-row", index))
                    .role(Role::Button)
                    .aria_label(format!("Edit {}", project.name))
                    .cursor_pointer()
                    .hover(|style| style.bg(colors.surface2))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open(&root, &name, window, cx)),
                    )
                    .child(icon(IconName::FolderGit2).text_color(colors.foreground_muted))
                    .child(row_text(
                        project.name.clone(),
                        Some(project.root.clone().into()),
                        cx,
                    ))
                    .child(
                        icon(IconName::ChevronRight)
                            .with_size(px(ICON_SM))
                            .text_color(colors.foreground_muted),
                    )
                    .into_any_element()
            })
            .collect();
        let body = if rows.is_empty() {
            card(cx)
                .child(div().p(px(SPACE_4)).child(note("No projects yet", cx)))
                .into_any_element()
        } else {
            card(cx)
                .id("project-list")
                .role(Role::List)
                .aria_label("Projects")
                .children(rows)
                .into_any_element()
        };
        scroll_page(
            "projects-pane",
            &self.scroll,
            column()
                .child(page_title("Projects", cx))
                .child(section("Projects", None, body, cx)),
            cx,
        )
        .into_any_element()
    }
}
