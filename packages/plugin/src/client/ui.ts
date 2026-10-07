import type { ComponentType, ReactNode, Ref } from "react";

export interface SettingsSectionProps {
  title: string;
  info?: ReactNode;
  trailing?: ReactNode;
  children: ReactNode;
  testID?: string;
}
export interface SettingsRowProps {
  label: string;
  hint?: string;
  error?: string | null;
  children?: ReactNode;
  testID?: string;
}
export interface SettingsSwitchProps extends SettingsRowProps {
  value: boolean;
  onValueChange(value: boolean): void;
  disabled?: boolean;
}
export interface SettingsSelectProps<Value extends string = string> extends SettingsRowProps {
  value: Value;
  options: readonly { label: string; value: Value }[];
  onValueChange(value: Value): void;
  disabled?: boolean;
}
export interface SettingsInputHandle {
  focus(): void;
  blur(): void;
  getText(): string;
  replaceText(text: string): void;
}
export interface SettingsInputProps extends SettingsRowProps {
  initialValue?: string;
  onChangeText(text: string): void;
  placeholder?: string;
  disabled?: boolean;
  secureTextEntry?: boolean;
  ref?: Ref<SettingsInputHandle>;
}
export interface SettingsActionProps extends SettingsRowProps {
  actionLabel: string;
  onPress(): void;
  disabled?: boolean;
}
export declare const SettingsGroup: ComponentType<SettingsSectionProps>;
export declare const SettingsSection: ComponentType<SettingsSectionProps>;
export declare const SettingsCard: ComponentType<{ children: ReactNode; testID?: string }>;
export declare const SettingsRow: ComponentType<SettingsRowProps>;
export declare const SettingsSwitch: ComponentType<SettingsSwitchProps>;
export declare function SettingsSelect<Value extends string>(
  props: SettingsSelectProps<Value>,
): ReactNode;
export declare const SettingsInput: ComponentType<SettingsInputProps>;
export declare const SettingsAction: ComponentType<SettingsActionProps>;

export interface ExternalLinkProps {
  href: string;
  children: ReactNode;
  accessibilityLabel?: string;
  testID?: string;
  onError?: (error: unknown) => void;
}
export declare const ExternalLink: ComponentType<ExternalLinkProps>;

/** Host controls for plugin-owned panels. */
export interface ButtonProps {
  children?: ReactNode;
  onPress(): void;
  disabled?: boolean;
  loading?: boolean;
  accessibilityLabel?: string;
  variant?: "default" | "secondary" | "outline" | "ghost" | "destructive";
  size?: "xs" | "sm" | "md" | "lg";
  leftIcon?: ReactNode;
  trailing?: ReactNode;
  onHoverIn?: import("react-native").PressableProps["onHoverIn"];
  onHoverOut?: import("react-native").PressableProps["onHoverOut"];
  style?: import("react-native").StyleProp<import("react-native").ViewStyle>;
  textStyle?: import("react-native").StyleProp<import("react-native").TextStyle>;
}
export declare const Button: ComponentType<ButtonProps>;
/** The host's Markdown renderer, including its link handling and text wrapping. */
export declare const Markdown: ComponentType<{
  text: string;
  compact?: boolean;
  enableHtmlish?: boolean;
  textStyle?: Pick<import("react-native").TextStyle, "color" | "fontSize" | "lineHeight">;
}>;
/** Command on macOS, Control elsewhere; false on touch-only clients. */
export declare function usePrimaryModifier(): boolean;

/** A run of code text; `style` names its syntax category, such as `"keyword"`. */
export interface SyntaxTokenData {
  text: string;
  style: string | null;
}
/**
 * Tokenizes `code` with the host highlighter, chosen by the extension of `filePath`.
 * Returns one token array per line, or null when the language is unsupported or the code is too large.
 */
export declare function tokenizeCode(code: string, filePath: string): SyntaxTokenData[][] | null;
/** Renders one token in the active theme's syntax color. Nest it inside a `Text`. */
export declare const SyntaxToken: ComponentType<{ token: SyntaxTokenData }>;

export interface HtmlFrameProps {
  /** A self-contained HTML document. Its scripts run in an opaque origin. */
  html: string;
  /**
   * A number fixes the height, and taller content scrolls inside the frame. `"auto"` grows the
   * frame to fit the page, up to 8000; a page sized to the frame, such as one with `100vh`,
   * stops growing and scrolls instead. Defaults to 360.
   */
  height?: number | "auto";
  /** Allows https scripts, styles, fonts, images, and fetch. Defaults to false: offline. */
  network?: boolean;
  /** Accessible title of the web frame. Defaults to "HTML". */
  title?: string;
  testID?: string;
}
/**
 * Renders HTML in the file preview's sandbox: an iframe with `sandbox="allow-scripts"` on web
 * and a locked-down WebView on native. The page gets the app theme as `--paseo-*` CSS variables.
 */
export declare const HtmlFrame: ComponentType<HtmlFrameProps>;
