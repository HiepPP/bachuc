import type { QueryClient } from "@tanstack/react-query";
import type { PluginRequirements } from "@getpaseo/protocol/messages";
import type {
  PluginAttachmentSourceContribution,
  PluginCleanup,
  PluginThemeContribution,
} from "@getpaseo/plugin";
import type {
  PluginCommandCenterItemContribution,
  PluginClientSlashCommandContribution,
  PluginComposerInterceptorContribution,
  PluginComposerPillContribution,
  PluginSidebarContribution,
  PluginSidebarProjectFilterContribution,
  PluginSidebarProjectMenuContribution,
  PluginSidebarSectionContribution,
  PluginSurfaceContribution,
  PluginSettingsScreenContribution,
  PluginTimelineRendererContribution,
  PluginWorkspaceHeaderSubtitleContribution,
  PluginComposerStopButtonContribution,
  PluginTimelineTransformerContribution,
  PluginPanelLocation,
  PluginWorkspacePanelContribution,
} from "@getpaseo/plugin/client";

export type EvaluatedPluginWorkspacePanelContribution = PluginWorkspacePanelContribution & {
  locations: readonly PluginPanelLocation[];
};

export interface EvaluatedPlugin {
  id: string;
  cleanup: PluginCleanup;
  surfaces: PluginSurfaceContribution[];
  settingsScreens: PluginSettingsScreenContribution[];
  sidebarItems: PluginSidebarContribution[];
  workspacePanels: EvaluatedPluginWorkspacePanelContribution[];
  commandCenterItems: PluginCommandCenterItemContribution[];
  clientSlashCommands: PluginClientSlashCommandContribution[];
  attachmentSources: PluginAttachmentSourceContribution[];
  themes: PluginThemeContribution[];
  timelineTransformers: PluginTimelineTransformerContribution[];
  timelineRenderers: PluginTimelineRendererContribution[];
  composerInterceptors?: PluginComposerInterceptorContribution[];
  sidebarProjectFilters?: PluginSidebarProjectFilterContribution[];
  sidebarProjectMenus?: PluginSidebarProjectMenuContribution[];
  sidebarSections?: PluginSidebarSectionContribution[];
  workspaceHeaderSubtitles?: PluginWorkspaceHeaderSubtitleContribution[];
  composerStopButtons?: PluginComposerStopButtonContribution[];
}

export interface InstalledPlugin extends EvaluatedPlugin {
  lifetime: AbortController;
  serverId: string;
  requirements?: PluginRequirements;
  clientBundle: string;
  queryClient: QueryClient;
}

export type {
  PluginAttachmentSourceContribution,
  PluginCommandCenterItemContribution,
  PluginClientSlashCommandContribution,
  PluginComposerPillContribution,
  PluginSidebarContribution,
  PluginSurfaceContribution,
  PluginSettingsScreenContribution,
  PluginThemeContribution,
  PluginTimelineRendererContribution,
  PluginTimelineTransformerContribution,
  PluginWorkspacePanelContribution,
};
