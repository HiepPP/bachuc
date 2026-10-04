import type { ComponentType } from "react";
import type { PaseoApi } from "@getpaseo/client";
import type { AgentTimelineItem } from "@getpaseo/protocol/agent-types";
import type { ZodType, input as ZodInput, output as ZodOutput } from "zod";
import type { PluginRpcContract } from "../rpc.js";
import type {
  PluginButtonRegistration,
  PluginHeaderButtonContribution,
  PluginComposerPillContribution,
} from "./buttons.js";
import type {
  PluginTheme,
  PluginWorkspaceSnapshot,
  PluginAgentSnapshot,
  PluginThemeContribution,
  PluginAttachmentSourceContribution,
  PluginTimelineTransformResult,
  PluginCleanup,
} from "../contracts.js";

export interface PluginHostProps {
  theme: PluginTheme;
  host: {
    id: string;
    label: string;
  };
  layout: {
    compact: boolean;
    platform: "ios" | "android" | "web";
  };
}

interface PluginNavigableHostProps extends PluginHostProps {
  /** Client-owned navigation. Undefined on older hosts; hide dependent affordances when absent. */
  readonly navigation?: {
    /** Present only on Electron. The browser runs locally; serverId selects workspace ownership. */
    readonly openBrowser?: (input: {
      readonly url: string;
      readonly workspaceId: string;
      readonly serverId?: string;
    }) => void;
    readonly openAgent: (input: {
      readonly agentId: string;
      readonly serverId?: string;
      /** Where the transcript opens. Defaults to its bottom. `latest-prompt` is web only. */
      readonly anchor?: "bottom" | "latest-prompt";
    }) => void;
    readonly openWorkspace: (input: {
      readonly workspaceId: string;
      readonly serverId?: string;
    }) => void;
  };
}

export interface PluginSurfaceProps extends PluginNavigableHostProps {}

export interface PluginIconProps {
  name: string;
  size?: number;
  color?: string;
}

export type PluginPanelLocation = "workspace" | "explorer";

export interface PluginOpenPanelOptions {
  location?: PluginPanelLocation;
}

interface PluginWorkspacePanelBase {
  id: string;
  title: string;
  icon: string;
  locations?: readonly PluginPanelLocation[];
}

export interface PluginWorkspacePanelProps extends PluginNavigableHostProps {
  context: "workspace";
  workspaceId: string;
}

export interface PluginAgentPanelProps extends PluginNavigableHostProps {
  context: "agent";
  workspaceId: string;
  agentId: string;
}

export interface PluginClientOpenPanelOptions extends PluginOpenPanelOptions {
  workspaceId: string;
  agentId?: string;
}

export interface PluginClientContext extends PluginCommandCapabilities {
  addSettingsScreen(contribution: PluginSettingsScreenContribution): PluginCleanup;
  addSurface(id: string, Component: ComponentType<PluginSurfaceProps>): PluginCleanup;
  addSidebarItem(contribution: PluginSidebarContribution): PluginCleanup;
  addWorkspacePanel(contribution: PluginWorkspacePanelContribution): PluginCleanup;
  addCommandCenterItem(contribution: PluginCommandCenterItemContribution): PluginCleanup;
  addSlashCommand(contribution: PluginClientSlashCommandContribution): PluginCleanup;
  addHeaderButton(contribution: PluginHeaderButtonContribution): PluginButtonRegistration;
  addComposerPill(contribution: PluginComposerPillContribution): PluginButtonRegistration;
  addAttachmentSource(contribution: PluginAttachmentSourceContribution): PluginCleanup;
  addTheme(contribution: PluginThemeContribution): PluginCleanup;
  addTimelineTransformer<ItemType extends AgentTimelineItem["type"]>(
    contribution: PluginTimelineTransformerContribution<ItemType>,
  ): PluginCleanup;
  addTimelineRenderer<Schema extends ZodType>(
    contribution: PluginTimelineRendererContribution<Schema>,
  ): PluginCleanup;
  /** Hides sidebar projects. Call the subscribed listener when the answer changes. */
  addSidebarProjectFilter(contribution: PluginSidebarProjectFilterContribution): PluginCleanup;
  /** Adds items to the sidebar project menus, before Remove. */
  addSidebarProjectMenuItems(contribution: PluginSidebarProjectMenuContribution): PluginCleanup;
  /** Renders a section above the sidebar footer. */
  addSidebarSection(contribution: PluginSidebarSectionContribution): PluginCleanup;
  /** Replaces the project name in the header of workspaces on this plugin's host. */
  addWorkspaceHeaderSubtitle(
    contribution: PluginWorkspaceHeaderSubtitleContribution,
  ): PluginCleanup;
  /** Runs before a composer on this plugin's host sends or queues a message. */
  addComposerInterceptor(contribution: PluginComposerInterceptorContribution): PluginCleanup;
  /** Opens the new workspace screen for a project directory, on this plugin's host by default. */
  openNewWorkspace(input: {
    cwd: string;
    projectId?: string;
    name?: string;
    serverId?: string;
  }): void;
  /** Replaces the text of an agent's composer on this plugin's host and focuses it. */
  setComposerText(input: { agentId: string; text: string }): void;
  /** Opens the Plugins page of host settings for this plugin's host. */
  openPluginsPage(): void;
  openPanel(id: string, options: PluginClientOpenPanelOptions): void;
}

export interface PluginSidebarProject {
  viewKey: string;
  name: string;
  serverIds: readonly string[];
  projectIds: readonly string[];
}

/** `activeServerId` is null when the sidebar shows all hosts. */
export interface PluginSidebarContext {
  activeServerId: string | null;
}

/** Pass `direction` when the filter moved to an adjacent view, so the host slides the list. */
export interface PluginSidebarFilterChange {
  direction?: 1 | -1;
}

export interface PluginSidebarProjectFilterContribution {
  id: string;
  subscribe(listener: (change?: PluginSidebarFilterChange) => void): () => void;
  isVisible(project: PluginSidebarProject, context: PluginSidebarContext): boolean;
  /**
   * Called for a horizontal trackpad swipe over the project list on desktop. `1` is a swipe
   * toward the next item, `-1` toward the previous one. One gesture calls it once.
   */
  onSwipe?(direction: 1 | -1, context: PluginSidebarContext): void;
  /** Replaces the sidebar's "Workspaces" heading while it returns a non-empty string. */
  getTitle?(context: PluginSidebarContext): string | null;
  /** Runs after Add Project succeeds, before navigation. Reject to keep the flow open for retry. */
  onProjectAdded?(project: PluginSidebarProject, context: PluginSidebarContext): Promise<void>;
}

export interface PluginSidebarProjectMenuItem {
  id: string;
  title: string;
  disabled?: boolean;
  /** Draws the menu's check, for the chosen value in a list of options. */
  checked?: boolean;
  /** Opens these items as a submenu instead of calling `onSelect`. */
  items?: readonly PluginSidebarProjectMenuItem[];
  onSelect?(): void | Promise<void>;
}

export interface PluginSidebarProjectMenuContribution {
  id: string;
  getItems(
    project: PluginSidebarProject,
    context: PluginSidebarContext,
  ): readonly PluginSidebarProjectMenuItem[];
}

export interface PluginSidebarSectionProps extends PluginHostProps, PluginSidebarContext {}

export interface PluginSidebarSectionContribution {
  id: string;
  Component: ComponentType<PluginSidebarSectionProps>;
}

export interface PluginWorkspaceHeaderSubtitleProps extends PluginHostProps {
  workspaceId: string;
  projectId: string;
  projectDisplayName: string;
}

export interface PluginWorkspaceHeaderSubtitleContribution {
  id: string;
  Component: ComponentType<PluginWorkspaceHeaderSubtitleProps>;
}

/** `agentId` is null for composers that create a new agent. */
export interface PluginComposerTarget {
  /** New workspace preferences, keyed by plugin ID. Absent on existing agents. */
  draftValues?: Readonly<Record<string, unknown>>;
  serverId: string;
  workspaceId: string | null;
  agentId: string | null;
}

/** `mod-enter` is Cmd+Enter on macOS and Ctrl+Enter elsewhere. */
export type PluginComposerSendKey = "enter" | "mod-enter";

export interface PluginComposerInterceptInput {
  target: PluginComposerTarget;
  text: string;
  action: "send" | "queue";
  /** The key that sent the message. Absent for the send button, dictation, and older hosts. */
  sendKey?: PluginComposerSendKey;
}

/** Return `{ text }` to change the message, `{ cancel: true }` to keep the draft, or nothing. */
export type PluginComposerInterceptResult = { text: string } | { cancel: true } | undefined;

export interface PluginComposerInterceptorContribution {
  id: string;
  intercept(
    input: PluginComposerInterceptInput,
  ): PluginComposerInterceptResult | Promise<PluginComposerInterceptResult>;
}

export type PluginClientContribution = (client: PluginClientContext) => PluginCleanup;

export type PluginWorkspacePanelContribution =
  | (PluginWorkspacePanelBase & {
      context: "workspace";
      Component: ComponentType<PluginWorkspacePanelProps>;
    })
  | (PluginWorkspacePanelBase & {
      context: "agent";
      Component: ComponentType<PluginAgentPanelProps>;
    });

export interface PluginSettingsScreenContribution {
  id: string;
  title: string;
  icon: string;
  Component: ComponentType<PluginSurfaceProps>;
}

export interface PluginSurfaceContribution {
  id: string;
  Component: ComponentType<PluginSurfaceProps>;
}

export interface PluginSidebarAction {
  /** `workspaceId` is the open workspace on this plugin's host, or null outside one. */
  onPress(context: { workspaceId: string | null }): void;
  /** Disables the row while no workspace on this plugin's host is open. */
  requiresWorkspace?: boolean;
}

export interface PluginSidebarContribution {
  id: string;
  title: string;
  icon: string;
  surface: string;
  /** Runs instead of opening `surface` when the row is pressed. */
  action?: PluginSidebarAction;
}

export type PluginTimelineTransformerContribution<
  ItemType extends AgentTimelineItem["type"] = AgentTimelineItem["type"],
> = ItemType extends AgentTimelineItem["type"]
  ? {
      id: string;
      query: {
        itemType: ItemType;
      };
      transform(input: {
        item: Extract<AgentTimelineItem, { type: ItemType }>;
        phase: "streaming" | "complete";
      }): PluginTimelineTransformResult | undefined;
    }
  : never;

export interface PluginTimelineItemProps<Data = unknown> extends PluginHostProps {
  agentId: string;
  item: {
    type: "plugin";
    kind: string;
    version: number;
    data: Data;
  };
  timestamp: Date;
}

export interface PluginTimelineRendererContribution<Schema extends ZodType = ZodType> {
  kind: string;
  version: number;
  schema: Schema;
  Component: ComponentType<PluginTimelineItemProps<ZodOutput<Schema>>>;
}

export interface PluginCommandCapabilities {
  paseo: PaseoApi;
  rpc<InputSchema extends ZodType, OutputSchema extends ZodType>(
    contract: PluginRpcContract<InputSchema, OutputSchema>,
    input: ZodInput<InputSchema>,
    /** Calls this plugin's installation on another configured, connected host. */
    options?: { serverId: string },
  ): Promise<ZodOutput<OutputSchema>>;
  /** Opens a surface. `pluginId` opens another plugin's surface; `serverId` picks its host. */
  openSurface(id: string, options?: { pluginId?: string; serverId?: string }): void;
  openSettings(id: string): void;
}

export interface PluginGlobalCommandContext extends PluginCommandCapabilities {
  context: "global";
}

export interface PluginWorkspaceCommandContext extends PluginCommandCapabilities {
  context: "workspace";
  workspace: PluginWorkspaceSnapshot;
  openPanel(id: string, options?: PluginOpenPanelOptions): void;
}

export interface PluginAgentCommandContext extends PluginCommandCapabilities {
  context: "agent";
  workspace: PluginWorkspaceSnapshot;
  agent: PluginAgentSnapshot;
  openPanel(id: string, options?: PluginOpenPanelOptions): void;
}

interface PluginCommandCenterItemBase {
  id: string;
  title: string;
  icon: string;
  keywords?: readonly string[];
}

export type PluginCommandCenterItemContribution =
  | (PluginCommandCenterItemBase & {
      context: "global";
      onSelect(context: PluginGlobalCommandContext): void | Promise<void>;
    })
  | (PluginCommandCenterItemBase & {
      context: "workspace";
      onSelect(context: PluginWorkspaceCommandContext): void | Promise<void>;
    })
  | (PluginCommandCenterItemBase & {
      context: "agent";
      onSelect(context: PluginAgentCommandContext): void | Promise<void>;
    });

interface PluginClientSlashCommandBase {
  name: string;
  description: string;
  argumentHint: string;
}

export type PluginClientSlashCommandContribution =
  | (PluginClientSlashCommandBase & {
      context: "workspace";
      onSubmit(context: PluginWorkspaceCommandContext & { args: string }): void | Promise<void>;
    })
  | (PluginClientSlashCommandBase & {
      context: "agent";
      onSubmit(context: PluginAgentCommandContext & { args: string }): void | Promise<void>;
    });

export type SettingsState<Schema extends ZodType> = (
  | { status: "loading" }
  | { status: "error"; error: string }
  | { status: "invalid"; error: string; revision: string }
  | { status: "ready"; values: ZodOutput<Schema>; revision: string }
) & {
  saving: boolean;
  saveError: string | null;
  /** Save an entire document against the revision currently displayed. Never throws. */
  save(values: ZodOutput<Schema>, revision: string): Promise<boolean>;
  reset(): Promise<boolean>;
  reload(): Promise<void>;
};
