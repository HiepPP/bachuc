import type { ComponentType } from "react";
import type { PluginHostProps } from "./contracts.js";

/** Local JSON preferences, transferred to the new agent through its creation environment. */
export interface PluginComposerDraft {
  id: string;
  get(): unknown;
  set(value: unknown): void;
}

export type PluginButtonContext =
  | { context: "workspace"; workspaceId: string }
  | { context: "agent"; workspaceId: string; agentId: string }
  | { context: "draft"; workspaceId: ""; draft: PluginComposerDraft };

export type PluginButtonIconProps = PluginHostProps &
  PluginButtonContext & { size: number; color: string };

export type PluginButtonContentProps = PluginHostProps & PluginButtonContext & { close(): void };

export type PluginButtonIcon = string | ComponentType<PluginButtonIconProps>;

export type PluginButtonLabelProps = PluginHostProps & PluginButtonContext;

/** A component returns the text for its own context, so one shared pill can differ per composer. */
export type PluginButtonLabel = string | ComponentType<PluginButtonLabelProps>;

export type PluginButtonBehavior =
  | {
      kind: "action";
      onPress(context?: PluginButtonContext): void | Promise<void>;
      /** On New workspace, a press shows this plugin's new workspace panel instead of onPress. */
      newWorkspacePanel?: string;
    }
  | { kind: "menu"; items: readonly PluginButtonMenuEntry[] }
  | {
      kind: "popover";
      Content: ComponentType<PluginButtonContentProps>;
      /** Drops the host padding so Content can draw its own edge-to-edge rows, like a menu. */
      flush?: boolean;
    };

export type PluginButtonMenuEntry =
  | { kind: "separator"; id: string }
  | {
      kind: "item";
      id: string;
      title: string;
      icon?: PluginButtonIcon;
      visible?: boolean;
      disabled?: boolean;
      behavior: PluginButtonBehavior;
    };

export interface PluginButton {
  title: string;
  icon: PluginButtonIcon;
  /** Omit for an icon-only header button. Composer pills use title when omitted. */
  label?: PluginButtonLabel;
  visible?: boolean;
  disabled?: boolean;
  behavior: PluginButtonBehavior;
}

export interface PluginButtonRegistration {
  /** Updates presentation in place. Supply a complete behavior to replace it. */
  update(patch: Partial<PluginComposerPillButton>): void;
  /** Idempotent. Updates after removal do nothing. */
  remove(): void;
}

export interface PluginHeaderButtonContribution {
  id: string;
  workspaceId: string;
  button: PluginButton;
}

/** A pill always shows its label or title, so its icon is optional. */
export type PluginComposerPillButton = Omit<PluginButton, "icon"> & {
  icon?: PluginButtonIcon;
  /** Hex color for the pill's icon, label, and border. Omit for the host's muted tone. */
  color?: string;
};

/** Omit `agentId` or `workspaceId` to show the pill on every matching agent composer. */
export interface PluginComposerPillContribution {
  id: string;
  /** Opt in to New workspace. Only unscoped pills can appear there. */
  showOnDraft?: boolean;
  /**
   * "toolbar" puts the pill beside the model selector. "corner" stacks it at the top-right corner
   * of the agent's pane. Compact layouts have room for neither, so the pill stays in the track bar.
   * Defaults to "track".
   */
  placement?: "track" | "toolbar" | "corner";
  workspaceId?: string;
  agentId?: string;
  button: PluginComposerPillButton;
}
