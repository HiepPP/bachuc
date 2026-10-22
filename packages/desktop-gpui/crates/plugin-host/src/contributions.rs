//! The contribution kinds a plugin adds in `NativePlugin::contribute`, except the composer
//! kinds (`composer`). Each kind is a plain struct of values and `Rc` closures, so it clones
//! cheaply and needs no lifetime.

use crate::composer::{AttachmentSource, ComposerInterceptor, ComposerPill, StopButton};
use crate::{Handler, ViewFactory};
use bachuc_icons::IconName;
use gpui_kit::{App, Hsla, SharedString, Task};
use serde_json::Value;
use std::collections::BTreeMap;
use std::rc::Rc;

/// A sidebar row that opens one of the plugin's surfaces (`PluginSidebarItemContribution`).
#[derive(Clone)]
pub struct SidebarItem {
    pub id: &'static str,
    pub title: SharedString,
    pub icon: IconName,
    pub surface: &'static str,
    pub action: Option<SidebarAction>,
}

/// The trailing action of a sidebar row.
#[derive(Clone)]
pub struct SidebarAction {
    /// Gets the active workspace id.
    pub on_press: Handler<Option<String>>,
    pub requires_workspace: bool,
    pub new_workspace_panel: Option<&'static str>,
}

/// A block the plugin draws in the sidebar.
#[derive(Clone)]
pub struct SidebarSection {
    pub id: &'static str,
    pub view: ViewFactory<()>,
}

/// One project row of the sidebar, as filters and menus see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarProject {
    pub view_key: String,
    pub name: String,
    pub project_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwipeDirection {
    Next,
    Previous,
}

/// Hides sidebar projects; with `title`, the sidebar shows the filter's name.
#[derive(Clone)]
pub struct ProjectFilter {
    pub id: &'static str,
    pub is_visible: Rc<dyn Fn(&SidebarProject, &App) -> bool>,
    pub title: Option<Rc<dyn Fn(&App) -> Option<SharedString>>>,
    pub on_swipe: Option<Rc<dyn Fn(SwipeDirection, &mut App)>>,
    pub on_project_added: Option<Rc<dyn Fn(SidebarProject, &mut App) -> Task<anyhow::Result<()>>>>,
}

/// Items the plugin adds to a project row's menu.
#[derive(Clone)]
pub struct ProjectMenu {
    pub id: &'static str,
    pub items: Rc<dyn Fn(&SidebarProject, &App) -> Vec<ProjectMenuItem>>,
}

/// One project menu item; with `items`, a submenu.
#[derive(Clone)]
pub struct ProjectMenuItem {
    pub id: SharedString,
    pub title: SharedString,
    pub disabled: bool,
    pub checked: bool,
    pub items: Vec<ProjectMenuItem>,
    pub on_select: Option<Handler<()>>,
}

/// Where a command center item or slash command applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandContext {
    Global,
    Workspace,
    Agent,
}

/// The ids a command gets when it runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandInput {
    pub workspace_id: Option<String>,
    pub agent_id: Option<String>,
}

#[derive(Clone)]
pub struct CommandCenterItem {
    pub id: &'static str,
    pub title: SharedString,
    pub icon: IconName,
    pub keywords: Vec<SharedString>,
    pub context: CommandContext,
    /// `None` shows the item always.
    pub visible: Option<Rc<dyn Fn(&App) -> bool>>,
    pub on_select: Handler<CommandInput>,
}

/// A composer slash command that runs in the client.
#[derive(Clone)]
pub struct PluginSlashCommand {
    pub name: &'static str,
    pub description: SharedString,
    pub argument_hint: SharedString,
    pub context: CommandContext,
    pub on_submit: Handler<SlashInput>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlashInput {
    pub workspace_id: Option<String>,
    pub agent_id: Option<String>,
    pub args: String,
}

/// A page under Settings.
#[derive(Clone)]
pub struct SettingsScreen {
    pub id: &'static str,
    pub title: SharedString,
    pub icon: IconName,
    pub view: ViewFactory<()>,
}

/// A page or overlay the plugin opens by id.
#[derive(Clone)]
pub struct Surface {
    pub id: &'static str,
    pub title: Option<SharedString>,
    pub icon: Option<IconName>,
    pub view: ViewFactory<SurfaceInput>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SurfaceInput {
    pub params: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SurfacePresentation {
    #[default]
    Page,
    Overlay,
}

/// A workspace tab panel.
#[derive(Clone)]
pub struct WorkspacePanel {
    pub id: &'static str,
    pub title: SharedString,
    pub icon: IconName,
    pub context: PanelContext,
    pub locations: Vec<PanelLocation>,
    pub view: ViewFactory<PanelInput>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelContext {
    Workspace,
    Agent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelLocation {
    Workspace,
    Explorer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelInput {
    pub workspace_id: String,
    pub agent_id: Option<String>,
}

/// A panel on the New workspace screen.
#[derive(Clone)]
pub struct NewWorkspacePanel {
    pub id: &'static str,
    pub title: SharedString,
    pub icon: IconName,
    pub view: ViewFactory<NewWorkspacePanelInput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewWorkspacePanelInput {
    pub project_id: String,
    pub cwd: String,
}

/// A keyboard shortcut. `combo` uses the host syntax, such as `Mod+G`.
#[derive(Clone)]
pub struct Shortcut {
    pub id: &'static str,
    pub combo: &'static str,
    pub surface: Option<&'static str>,
    /// Gets the active workspace id.
    pub on_press: Handler<Option<String>>,
}

/// A line under the workspace header title.
#[derive(Clone)]
pub struct HeaderSubtitle {
    pub id: &'static str,
    pub view: ViewFactory<HeaderSubtitleInput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderSubtitleInput {
    pub workspace_id: String,
    pub project_id: String,
    pub project_display_name: String,
    pub project_root_path: Option<String>,
}

/// Turns one timeline item of `item_type` into plugin items. A pure function of its input,
/// so the timeline caches results by entry and `PluginHost::timeline_revision`.
#[derive(Clone)]
pub struct TimelineTransformer {
    pub id: &'static str,
    pub item_type: &'static str,
    pub transform: Rc<dyn Fn(&TimelineTransformInput<'_>) -> Option<TimelineTransformResult>>,
}

#[derive(Clone, Copy, Debug)]
pub struct TimelineTransformInput<'a> {
    pub agent_id: &'a str,
    pub labels: &'a BTreeMap<String, String>,
    pub item: &'a Value,
    pub phase: TimelinePhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelinePhase {
    Streaming,
    Complete,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineTransformResult {
    pub items: Vec<PluginTimelineItem>,
    /// `None` hides the source item.
    pub source: Option<SourcePlacement>,
}

/// Where the source item stays next to the plugin items; `text` replaces its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePlacement {
    pub position: SourcePosition,
    pub text: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePosition {
    First,
    Last,
}

/// A plugin data item in the timeline, drawn by the renderer of its `kind` and `version`.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginTimelineItem {
    pub id: Option<String>,
    pub kind: String,
    pub version: u32,
    pub data: Value,
}

#[derive(Clone)]
pub struct TimelineRenderer {
    pub kind: &'static str,
    pub version: u32,
    pub view: ViewFactory<TimelineRenderInput>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineRenderInput {
    pub agent_id: String,
    pub item: PluginTimelineItem,
    pub timestamp: Option<String>,
}

/// An action on selected assistant text.
#[derive(Clone)]
pub struct SelectionAction {
    pub id: &'static str,
    pub title: SharedString,
    pub on_select: Handler<AssistantSelection>,
}

/// Runs a result action on the selected text and returns the text the host shows in its card.
/// It takes no `&mut Window`: `ctx.rpc` needs only `App`.
pub type SelectionResultHandler =
    Rc<dyn Fn(AssistantSelection, &mut App) -> Task<anyhow::Result<String>>>;

/// An action on selected assistant text whose answer is text: the host draws a card at the
/// selection with `loading` while the task runs, then the text as markdown or the error
/// (ADR-0063).
#[derive(Clone)]
pub struct SelectionResultAction {
    pub id: &'static str,
    pub title: SharedString,
    /// The card body while the task runs.
    pub loading: SharedString,
    pub run: SelectionResultHandler,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistantSelection {
    pub workspace_id: Option<String>,
    pub agent_id: String,
    pub message_id: Option<String>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeContribution {
    pub id: &'static str,
    pub name: SharedString,
    pub appearance: ThemeAppearance,
    pub colors: PluginThemeColors,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeAppearance {
    Light,
    Dark,
}

/// `PluginThemeColorsSchema` (`packages/plugin/src/contracts.ts`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PluginThemeColors {
    pub background: Hsla,
    pub foreground: Hsla,
    pub raised: Hsla,
    pub control: Hsla,
    pub border: Hsla,
    pub muted_foreground: Hsla,
    pub ring: Hsla,
    pub accent: Option<Hsla>,
}

/// What one plugin adds while it is active, in `add_*` call order.
#[derive(Clone, Default)]
pub struct Contributions {
    pub(crate) composer_pills: Vec<ComposerPill>,
    pub(crate) composer_interceptors: Vec<ComposerInterceptor>,
    pub(crate) stop_buttons: Vec<StopButton>,
    pub(crate) attachment_sources: Vec<AttachmentSource>,
    pub(crate) sidebar_items: Vec<SidebarItem>,
    pub(crate) sidebar_sections: Vec<SidebarSection>,
    pub(crate) project_filters: Vec<ProjectFilter>,
    pub(crate) project_menus: Vec<ProjectMenu>,
    pub(crate) command_items: Vec<CommandCenterItem>,
    pub(crate) slash_commands: Vec<PluginSlashCommand>,
    pub(crate) settings_screens: Vec<SettingsScreen>,
    pub(crate) surfaces: Vec<Surface>,
    pub(crate) workspace_panels: Vec<WorkspacePanel>,
    pub(crate) new_workspace_panels: Vec<NewWorkspacePanel>,
    pub(crate) shortcuts: Vec<Shortcut>,
    pub(crate) header_subtitles: Vec<HeaderSubtitle>,
    pub(crate) timeline_transformers: Vec<TimelineTransformer>,
    pub(crate) timeline_renderers: Vec<TimelineRenderer>,
    pub(crate) selection_actions: Vec<SelectionAction>,
    pub(crate) selection_result_actions: Vec<SelectionResultAction>,
    pub(crate) themes: Vec<ThemeContribution>,
    pub(crate) reading_look: Option<bachuc_theme::ReadingLook>,
}

impl Contributions {
    pub fn add_composer_pill(&mut self, pill: ComposerPill) {
        self.composer_pills.push(pill);
    }

    pub fn add_composer_interceptor(&mut self, interceptor: ComposerInterceptor) {
        self.composer_interceptors.push(interceptor);
    }

    pub fn add_stop_button(&mut self, button: StopButton) {
        self.stop_buttons.push(button);
    }

    pub fn add_attachment_source(&mut self, source: AttachmentSource) {
        self.attachment_sources.push(source);
    }

    pub fn add_sidebar_item(&mut self, item: SidebarItem) {
        self.sidebar_items.push(item);
    }

    pub fn add_sidebar_section(&mut self, section: SidebarSection) {
        self.sidebar_sections.push(section);
    }

    pub fn add_project_filter(&mut self, filter: ProjectFilter) {
        self.project_filters.push(filter);
    }

    pub fn add_project_menu(&mut self, menu: ProjectMenu) {
        self.project_menus.push(menu);
    }

    pub fn add_command_item(&mut self, item: CommandCenterItem) {
        self.command_items.push(item);
    }

    pub fn add_slash_command(&mut self, command: PluginSlashCommand) {
        self.slash_commands.push(command);
    }

    pub fn add_settings_screen(&mut self, screen: SettingsScreen) {
        self.settings_screens.push(screen);
    }

    pub fn add_surface(&mut self, surface: Surface) {
        self.surfaces.push(surface);
    }

    pub fn add_workspace_panel(&mut self, panel: WorkspacePanel) {
        self.workspace_panels.push(panel);
    }

    pub fn add_new_workspace_panel(&mut self, panel: NewWorkspacePanel) {
        self.new_workspace_panels.push(panel);
    }

    pub fn add_shortcut(&mut self, shortcut: Shortcut) {
        self.shortcuts.push(shortcut);
    }

    pub fn add_header_subtitle(&mut self, subtitle: HeaderSubtitle) {
        self.header_subtitles.push(subtitle);
    }

    pub fn add_timeline_transformer(&mut self, transformer: TimelineTransformer) {
        self.timeline_transformers.push(transformer);
    }

    pub fn add_timeline_renderer(&mut self, renderer: TimelineRenderer) {
        self.timeline_renderers.push(renderer);
    }

    pub fn add_selection_action(&mut self, action: SelectionAction) {
        self.selection_actions.push(action);
    }

    pub fn add_selection_result_action(&mut self, action: SelectionResultAction) {
        self.selection_result_actions.push(action);
    }

    pub fn add_theme(&mut self, theme: ThemeContribution) {
        self.themes.push(theme);
    }

    /// Sets the chat column width and reply line height while the plugin is active. With
    /// several plugins, the last one registered wins.
    pub fn set_reading_look(&mut self, look: bachuc_theme::ReadingLook) {
        self.reading_look = Some(look);
    }
}
