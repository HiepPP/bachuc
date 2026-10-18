//! The contributions of every active plugin, by kind, for the views.

use crate::composer::{AttachmentSource, ComposerInterceptor, ComposerPill, StopButton};
use crate::contributions::{
    CommandCenterItem, Contributions, HeaderSubtitle, NewWorkspacePanel, PluginSlashCommand,
    ProjectFilter, ProjectMenu, SelectionAction, SettingsScreen, Shortcut, SidebarItem,
    SidebarSection, Surface, ThemeContribution, TimelineRenderer, TimelineTransformer,
    WorkspacePanel,
};
use std::collections::HashMap;

/// One contribution with the plugin that added it.
#[derive(Clone)]
pub struct Contributed<T> {
    pub plugin_id: &'static str,
    pub value: T,
}

/// Every active contribution in registration order: plugin by plugin in `init` order, then
/// in `add_*` call order. Rebuilt in O(contributions) when the active set changes.
#[derive(Clone, Default)]
pub struct ContributionIndex {
    composer_pills: Vec<Contributed<ComposerPill>>,
    composer_interceptors: Vec<Contributed<ComposerInterceptor>>,
    stop_buttons: Vec<Contributed<StopButton>>,
    attachment_sources: Vec<Contributed<AttachmentSource>>,
    sidebar_items: Vec<Contributed<SidebarItem>>,
    sidebar_sections: Vec<Contributed<SidebarSection>>,
    project_filters: Vec<Contributed<ProjectFilter>>,
    project_menus: Vec<Contributed<ProjectMenu>>,
    command_items: Vec<Contributed<CommandCenterItem>>,
    slash_commands: Vec<Contributed<PluginSlashCommand>>,
    settings_screens: Vec<Contributed<SettingsScreen>>,
    surfaces: Vec<Contributed<Surface>>,
    workspace_panels: Vec<Contributed<WorkspacePanel>>,
    new_workspace_panels: Vec<Contributed<NewWorkspacePanel>>,
    shortcuts: Vec<Contributed<Shortcut>>,
    header_subtitles: Vec<Contributed<HeaderSubtitle>>,
    selection_actions: Vec<Contributed<SelectionAction>>,
    themes: Vec<Contributed<ThemeContribution>>,
    reading_look: Option<bachuc_theme::ReadingLook>,
    /// Transformers by item type, so the timeline's per-entry lookup allocates nothing.
    transformers: HashMap<&'static str, Vec<Contributed<TimelineTransformer>>>,
    /// Renderers by kind, then version; the first of a kind and version wins.
    renderers: HashMap<&'static str, Vec<Contributed<TimelineRenderer>>>,
}

fn tag<T: Clone>(plugin_id: &'static str, values: &[T], into: &mut Vec<Contributed<T>>) {
    into.extend(values.iter().map(|value| Contributed {
        plugin_id,
        value: value.clone(),
    }));
}

impl ContributionIndex {
    /// Indexes `plugins` in the order given, which is registration order.
    pub fn build<'a>(plugins: impl IntoIterator<Item = (&'static str, &'a Contributions)>) -> Self {
        let mut index = Self::default();
        for (id, plugin) in plugins {
            tag(id, &plugin.composer_pills, &mut index.composer_pills);
            tag(
                id,
                &plugin.composer_interceptors,
                &mut index.composer_interceptors,
            );
            tag(
                id,
                &plugin.attachment_sources,
                &mut index.attachment_sources,
            );
            tag(id, &plugin.sidebar_items, &mut index.sidebar_items);
            tag(id, &plugin.sidebar_sections, &mut index.sidebar_sections);
            tag(id, &plugin.project_filters, &mut index.project_filters);
            tag(id, &plugin.project_menus, &mut index.project_menus);
            tag(id, &plugin.command_items, &mut index.command_items);
            tag(id, &plugin.slash_commands, &mut index.slash_commands);
            tag(id, &plugin.settings_screens, &mut index.settings_screens);
            tag(id, &plugin.surfaces, &mut index.surfaces);
            tag(id, &plugin.workspace_panels, &mut index.workspace_panels);
            tag(
                id,
                &plugin.new_workspace_panels,
                &mut index.new_workspace_panels,
            );
            tag(id, &plugin.shortcuts, &mut index.shortcuts);
            tag(id, &plugin.header_subtitles, &mut index.header_subtitles);
            tag(id, &plugin.selection_actions, &mut index.selection_actions);
            tag(id, &plugin.themes, &mut index.themes);
            if plugin.reading_look.is_some() {
                index.reading_look = plugin.reading_look;
            }
            for button in &plugin.stop_buttons {
                if let Some(first) = index.stop_buttons.first() {
                    log::warn!(
                        "plugin {id}: stop button {} ignored, {} of plugin {} is first",
                        button.id,
                        first.value.id,
                        first.plugin_id
                    );
                    continue;
                }
                index.stop_buttons.push(Contributed {
                    plugin_id: id,
                    value: button.clone(),
                });
            }
            for transformer in &plugin.timeline_transformers {
                index
                    .transformers
                    .entry(transformer.item_type)
                    .or_default()
                    .push(Contributed {
                        plugin_id: id,
                        value: transformer.clone(),
                    });
            }
            for renderer in &plugin.timeline_renderers {
                let versions = index.renderers.entry(renderer.kind).or_default();
                if let Some(first) = versions
                    .iter()
                    .find(|first| first.value.version == renderer.version)
                {
                    log::warn!(
                        "plugin {id}: renderer {} v{} ignored, plugin {} registered it first",
                        renderer.kind,
                        renderer.version,
                        first.plugin_id
                    );
                    continue;
                }
                versions.push(Contributed {
                    plugin_id: id,
                    value: renderer.clone(),
                });
            }
        }
        index
    }

    pub fn composer_pills(&self) -> &[Contributed<ComposerPill>] {
        &self.composer_pills
    }

    pub fn composer_interceptors(&self) -> &[Contributed<ComposerInterceptor>] {
        &self.composer_interceptors
    }

    /// At most one: the first stop button registered.
    pub fn stop_buttons(&self) -> &[Contributed<StopButton>] {
        &self.stop_buttons
    }

    pub fn attachment_sources(&self) -> &[Contributed<AttachmentSource>] {
        &self.attachment_sources
    }

    pub fn sidebar_items(&self) -> &[Contributed<SidebarItem>] {
        &self.sidebar_items
    }

    pub fn sidebar_sections(&self) -> &[Contributed<SidebarSection>] {
        &self.sidebar_sections
    }

    pub fn project_filters(&self) -> &[Contributed<ProjectFilter>] {
        &self.project_filters
    }

    pub fn project_menus(&self) -> &[Contributed<ProjectMenu>] {
        &self.project_menus
    }

    pub fn command_items(&self) -> &[Contributed<CommandCenterItem>] {
        &self.command_items
    }

    pub fn slash_commands(&self) -> &[Contributed<PluginSlashCommand>] {
        &self.slash_commands
    }

    pub fn settings_screens(&self) -> &[Contributed<SettingsScreen>] {
        &self.settings_screens
    }

    pub fn surfaces(&self) -> &[Contributed<Surface>] {
        &self.surfaces
    }

    pub fn workspace_panels(&self) -> &[Contributed<WorkspacePanel>] {
        &self.workspace_panels
    }

    pub fn new_workspace_panels(&self) -> &[Contributed<NewWorkspacePanel>] {
        &self.new_workspace_panels
    }

    pub fn shortcuts(&self) -> &[Contributed<Shortcut>] {
        &self.shortcuts
    }

    pub fn header_subtitles(&self) -> &[Contributed<HeaderSubtitle>] {
        &self.header_subtitles
    }

    pub fn selection_actions(&self) -> &[Contributed<SelectionAction>] {
        &self.selection_actions
    }

    pub fn themes(&self) -> &[Contributed<ThemeContribution>] {
        &self.themes
    }

    /// The chat reading layout of the last plugin that set one.
    pub fn reading_look(&self) -> Option<bachuc_theme::ReadingLook> {
        self.reading_look
    }

    /// The transformers of one timeline item type, in registration order; empty for a type
    /// no plugin transforms.
    pub fn transformers_for(&self, item_type: &str) -> &[Contributed<TimelineTransformer>] {
        self.transformers
            .get(item_type)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn renderer_for(&self, kind: &str, version: u32) -> Option<&Contributed<TimelineRenderer>> {
        self.renderers
            .get(kind)?
            .iter()
            .find(|renderer| renderer.value.version == version)
    }

    pub fn surface(&self, plugin_id: &str, surface_id: &str) -> Option<&Contributed<Surface>> {
        self.surfaces
            .iter()
            .find(|surface| surface.plugin_id == plugin_id && surface.value.id == surface_id)
    }
}
