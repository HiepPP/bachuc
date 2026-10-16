//! `bachuc-plugin-host`: the host of the native plugin clients (ADR-0045).
//!
//! Each plugin client is a crate in `packages/desktop-gpui/plugins/<id>` named
//! `bachuc-plugin-<id>` that exports `pub fn plugin() -> Box<dyn NativePlugin>`. The app passes
//! those boxes to `init`. A plugin is active while the active host lists it enabled and
//! running; then its `contribute` runs and the views read its contributions from the
//! `ContributionIndex`.
//!
//! The contract follows `packages/plugin/src/client/contracts.ts`. View crates and plugin
//! crates depend on this crate, so it never depends on a view crate.

mod composer;
mod context;
mod contributions;
mod host;
mod index;
mod settings;
pub mod ui;

pub use composer::{
    AttachmentItem, AttachmentPicker, AttachmentSource, ComposerAttachment, ComposerDraft,
    ComposerInterceptor, ComposerPill, ComposerTarget, InterceptInput, InterceptOutcome,
    PillBehavior, PillPlacement, PillState, SelectOption, SendAction, SendKey, ShowOnDraft,
    StopButton, StopButtonProps, decode_attachment_search, pill_applies, search_attachments,
};
pub use context::{
    AgentAnchor, MessageLevel, NewWorkspaceRequest, OpenSurfaceOptions, PanelTarget, PluginContext,
    TimelinePassage,
};
pub use contributions::{
    AssistantSelection, CommandCenterItem, CommandContext, CommandInput, Contributions,
    HeaderSubtitle, HeaderSubtitleInput, NewWorkspacePanel, NewWorkspacePanelInput, PanelContext,
    PanelInput, PanelLocation, PluginSlashCommand, PluginThemeColors, PluginTimelineItem,
    ProjectFilter, ProjectMenu, ProjectMenuItem, SelectionAction, SettingsScreen, Shortcut,
    SidebarAction, SidebarItem, SidebarProject, SidebarSection, SlashInput, SourcePlacement,
    SourcePosition, Surface, SurfaceInput, SurfacePresentation, SwipeDirection, ThemeAppearance,
    ThemeContribution, TimelinePhase, TimelineRenderInput, TimelineRenderer,
    TimelineTransformInput, TimelineTransformResult, TimelineTransformer, WorkspacePanel,
};
pub use host::{PluginHost, PluginHostEvent, active_plugin_ids, host, init};
pub use index::{Contributed, ContributionIndex};
pub use settings::{PluginSettings, SettingsState};

use gpui_kit::{AnyView, App, Window};
use std::rc::Rc;

/// A press, select, or submit handler of a contribution.
pub type Handler<I> = Rc<dyn Fn(I, &mut Window, &mut App)>;
/// Builds a contribution's view. The consumer caches the view per contribution and input.
pub type ViewFactory<I> = Rc<dyn Fn(I, &mut Window, &mut App) -> AnyView>;
/// Closes the popover that a pill opened.
pub type PopoverClose = Rc<dyn Fn(&mut Window, &mut App)>;

/// One plugin client. The trait stays object-safe, so the app holds `Box<dyn NativePlugin>`.
pub trait NativePlugin {
    /// The plugin id, the same id as in the daemon's `plugin.list`.
    fn id(&self) -> &'static str;

    /// Runs each time the plugin becomes active on a host. Dropping `contributions`
    /// deactivates it, so entities and tasks its closures hold drop with them; there is no
    /// cleanup callback.
    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App);
}
