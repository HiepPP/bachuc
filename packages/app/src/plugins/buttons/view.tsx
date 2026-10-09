import { PluginClientStateProvider } from "@getpaseo/plugin/client/host";
import type {
  PluginButtonBehavior,
  PluginButtonIcon,
  PluginButtonLabel,
  PluginButtonMenuEntry,
  PluginHostProps,
} from "@getpaseo/plugin/client";
import type { PluginTheme } from "@getpaseo/plugin";
import { useCallback, useId, useMemo, useSyncExternalStore, type ReactNode } from "react";
import { Platform, Pressable, Text, View, useWindowDimensions } from "react-native";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import { AlertCircle, ChevronDown, MoreHorizontal } from "lucide-react-native";
import { useTranslation } from "react-i18next";
import {
  MenuRoot,
  MenuTrigger,
  MenuSurface,
  MenuItem,
  MenuSeparator,
  MenuSubTrigger,
  useMenuContext,
  type MenuPageDefinition,
} from "@/components/ui/menu";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { HEADER_CONTROL_HEIGHT } from "@/components/ui/control-geometry";
import {
  iconButtonChromeStyle,
  type IconButtonChromeState,
} from "@/components/ui/icon-button-chrome";
import { useComposerPillStyles } from "@/composer/pill-styles";
import { useIsCompactFormFactor } from "@/constants/layout";
import { ToastApiProvider, useToast } from "@/contexts/toast-context";
import { useHostRuntimeClient, useHosts } from "@/runtime/host-runtime";
import type { Theme } from "@/styles/theme";
import { createPluginClientStateSource } from "../client-state/source";
import {
  newWorkspacePanelKey,
  useNewWorkspaceSidePanelStore,
} from "../new-workspace-side-panel/store";
import { Icon } from "../icons";
import { PluginRuntimeBoundary } from "../runtime-boundary";
import { SurfaceErrorBoundary } from "../surface-error-boundary";
import { toPluginTheme } from "../theme";
import {
  buttonInToolbar,
  buttonSlot,
  type ComposerPillSlot,
  buttonMatches,
  resolveButtonForContext,
  type RegisteredPluginButton,
} from "./model";
import { pluginButtonStore } from "./store";
import type { PluginComposerDraftState } from "../composer/draft";

interface ButtonView {
  entry: RegisteredPluginButton;
  props: PluginHostProps & RegisteredPluginButton["context"];
  client: NonNullable<ReturnType<typeof useHostRuntimeClient>>;
  state: ReturnType<typeof createPluginClientStateSource>;
  toast: ReturnType<typeof useToast>;
}

const ROOT_PATH: readonly string[] = [];

const pluginThemeMapping = (theme: Theme) => ({ theme: toPluginTheme(theme) });
const mutedIconMapping = (theme: Theme) => ({ color: theme.colors.foregroundMuted });
const errorIconMapping = (theme: Theme) => ({ color: theme.colors.statusDanger });
const ThemedMoreIcon = withUnistyles(MoreHorizontal);
const ThemedErrorIcon = withUnistyles(AlertCircle);

function headerButtonStyle(compact: boolean, state: IconButtonChromeState, disabled = false) {
  if (compact) return iconButtonChromeStyle({ size: "large", state, disabled });
  return [
    styles.headerButton,
    styles.button,
    (state.hovered || state.pressed || state.open) && styles.active,
    disabled && styles.disabled,
  ];
}

function resolvePlatform(): PluginHostProps["layout"]["platform"] {
  if (Platform.OS === "ios") return "ios";
  if (Platform.OS === "android") return "android";
  return "web";
}

// These providers live inside the surface content as well as around its trigger. Native sheets
// teleport their children, so providers around MenuRoot alone cannot reach the plugin body.
function ButtonEnvironment({ view, children }: { view: ButtonView; children: ReactNode }) {
  return (
    <ToastApiProvider api={view.toast}>
      <PluginRuntimeBoundary plugin={view.entry.installation} client={view.client}>
        <PluginClientStateProvider source={view.state}>{children}</PluginClientStateProvider>
      </PluginRuntimeBoundary>
    </ToastApiProvider>
  );
}

function ButtonIcon({
  view,
  icon,
  color = view.props.theme.colors.foregroundMuted,
}: {
  view: ButtonView;
  icon: PluginButtonIcon;
  color?: string;
}) {
  const size = view.entry.placement === "composer" ? 14 : 16;
  return (
    <View
      style={styles.icon}
      pointerEvents="none"
      accessible={false}
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
    >
      {typeof icon === "string" ? (
        <Icon name={icon} size={size} color={color} />
      ) : (
        renderCustomIcon(icon, view, size, color)
      )}
    </View>
  );
}

function renderCustomIcon(
  CustomIcon: Exclude<PluginButtonIcon, string>,
  view: ButtonView,
  size: number,
  color: string,
) {
  return <CustomIcon {...view.props} size={size} color={color} />;
}

function renderCustomLabel(CustomLabel: Exclude<PluginButtonLabel, string>, view: ButtonView) {
  return <CustomLabel {...view.props} />;
}

function pressButton(view: ButtonView, path: readonly string[]) {
  const { behavior } = view.entry.button;
  const panelId = behavior.kind === "action" ? behavior.newWorkspacePanel : undefined;
  // The new workspace side panel exists on wide layouts only; compact ones run onPress.
  if (
    path.length === 0 &&
    panelId &&
    view.entry.context.context === "draft" &&
    !view.props.layout.compact
  ) {
    useNewWorkspaceSidePanelStore
      .getState()
      .show(newWorkspacePanelKey(view.entry.installation.id, panelId));
    return;
  }
  void pluginButtonStore.run(view.entry.key, path, view.entry.context).catch((error: unknown) => {
    view.toast.error(error instanceof Error ? error.message : String(error));
  });
}

function visibleMenuItems(
  items: readonly PluginButtonMenuEntry[],
): readonly PluginButtonMenuEntry[] {
  const visible: PluginButtonMenuEntry[] = [];
  for (const item of items) {
    if (item.kind === "item" && item.visible === false) continue;
    if (item.kind === "separator" && (visible.length === 0 || visible.at(-1)?.kind === "separator"))
      continue;
    visible.push(item);
  }
  if (visible.at(-1)?.kind === "separator") visible.pop();
  return visible;
}

function ButtonMenuItem({
  view,
  title,
  icon,
  behavior,
  disabled,
  path,
  pageId,
}: {
  view: ButtonView;
  title: string;
  icon?: PluginButtonIcon;
  behavior: PluginButtonBehavior;
  disabled: boolean;
  path: readonly string[];
  pageId: string;
}) {
  const select = useCallback(() => pressButton(view, path), [view, path]);
  const leading = useMemo(
    () => (icon ? <ButtonIcon view={view} icon={icon} /> : null),
    [icon, view],
  );
  if (behavior.kind !== "action") {
    return (
      <MenuSubTrigger id={pageId} leading={leading} disabled={disabled}>
        {title}
      </MenuSubTrigger>
    );
  }
  return (
    <MenuItem onSelect={select} leading={leading} disabled={disabled}>
      {title}
    </MenuItem>
  );
}

function ButtonBody({
  view,
  behavior,
  path,
}: {
  view: ButtonView;
  behavior: PluginButtonBehavior;
  path: readonly string[];
}) {
  const menu = useMenuContext("PluginButton");
  const close = useCallback(() => menu.setOpen(false), [menu]);
  if (behavior.kind === "popover") {
    const Content = behavior.Content;
    return (
      <View style={behavior.flush ? undefined : styles.content}>
        <Content {...view.props} close={close} />
      </View>
    );
  }
  if (behavior.kind !== "menu") return null;
  return visibleMenuItems(behavior.items).map((item) => {
    if (item.kind === "separator") return <MenuSeparator key={item.id} />;
    const itemPath = [...path, item.id];
    return (
      <ButtonMenuItem
        key={item.id}
        view={view}
        title={item.title}
        icon={item.icon}
        behavior={item.behavior}
        disabled={view.entry.pending || view.entry.button.disabled || item.disabled === true}
        path={itemPath}
        pageId={buttonPageId(view, itemPath)}
      />
    );
  });
}

function ButtonSurfaceBody({
  view,
  behavior,
  path,
}: {
  view: ButtonView;
  behavior: PluginButtonBehavior;
  path: readonly string[];
}) {
  return (
    <SurfaceErrorBoundary installation={view.entry.installation} Surface={behavior}>
      <ButtonEnvironment view={view}>
        <ButtonBody view={view} behavior={behavior} path={path} />
      </ButtonEnvironment>
    </SurfaceErrorBoundary>
  );
}

function buttonPageId(view: ButtonView, path: readonly string[]): string {
  return [view.entry.key, ...path].join("/");
}

function buttonPages(
  view: ButtonView,
  behavior: PluginButtonBehavior,
  path: readonly string[] = [],
): MenuPageDefinition[] {
  if (behavior.kind !== "menu") return [];
  return behavior.items.flatMap((item) => {
    if (item.kind === "separator" || item.visible === false || item.behavior.kind === "action")
      return [];
    const itemPath = [...path, item.id];
    return [
      {
        id: buttonPageId(view, itemPath),
        title: item.title,
        hoverIntent: item.behavior.kind === "menu",
        content: <ButtonSurfaceBody view={view} behavior={item.behavior} path={itemPath} />,
      },
      ...buttonPages(view, item.behavior, itemPath),
    ];
  });
}

function renderButtonIcon(view: ButtonView) {
  const { icon } = view.entry.button;
  return icon === undefined ? null : <ButtonIcon view={view} icon={icon} color={pillColor(view)} />;
}

function resolveLabel(view: ButtonView, composer: boolean): PluginButtonLabel | undefined {
  const { label, title } = view.entry.button;
  if (composer) return label ?? title;
  return view.props.layout.compact ? undefined : label;
}

/** Only a composer pill takes its plugin's color; header buttons keep the host tone. */
function pillColor(view: ButtonView): string | undefined {
  return view.entry.placement === "composer" ? view.entry.button.color : undefined;
}

function resolveLabelStyle(composer: boolean, toolbar: boolean) {
  if (toolbar) return styles.toolbarLabel;
  return composer ? styles.composerLabel : styles.label;
}

function ButtonControl({ view }: { view: ButtonView }) {
  const { entry, props } = view;
  const { button } = entry;
  const composer = entry.placement === "composer";
  const toolbar = buttonInToolbar(entry, props.layout.compact);
  const corner = buttonSlot(entry, props.layout.compact) === "corner";
  const color = pillColor(view);
  const disabled = button.disabled || entry.pending;
  const expanded = button.behavior.kind !== "action";
  const chevron = (toolbar || !composer) && !props.layout.compact && expanded;
  const label = resolveLabel(view, composer);
  const press = useCallback(() => pressButton(view, []), [view]);
  const composerPillStyles = useComposerPillStyles();
  let contextKey: string | undefined;
  if (entry.context.context === "agent") contextKey = entry.context.agentId;
  if (entry.context.context === "draft") contextKey = entry.context.draft.id;
  // The same agent's pill can be mounted more than once, and a hidden copy measures its trigger
  // at 0,0, which drew a second menu in the window's top-left corner. Only the pressed copy opens.
  const owner = useId();
  const open = entry.open && entry.openOwner === owner;
  const setOpen = useCallback(
    (next: boolean) => pluginButtonStore.setOpen(entry.key, next, contextKey, owner),
    [contextKey, entry.key, owner],
  );
  const buttonStyle = useCallback(
    ({ hovered, pressed }: { hovered?: boolean; pressed: boolean }) =>
      composer
        ? [
            toolbar
              ? styles.toolbarButton
              : [composerPillStyles.body, corner ? styles.cornerButton : styles.button],
            (hovered || pressed || open) && styles.active,
            disabled && styles.disabled,
            color !== undefined && { borderColor: color },
          ]
        : headerButtonStyle(props.layout.compact, { hovered, pressed, open }, disabled),
    [composer, toolbar, composerPillStyles, corner, color, disabled, open, props.layout.compact],
  );
  const labelStyle = useMemo(
    () => [resolveLabelStyle(composer, toolbar), color !== undefined && { color }],
    [composer, toolbar, color],
  );
  const contents = (
    <>
      {entry.pending ? (
        <View style={styles.icon}>
          <LoadingSpinner size={14} color={props.theme.colors.foregroundMuted} />
        </View>
      ) : (
        renderButtonIcon(view)
      )}
      {label ? (
        <Text numberOfLines={1} style={labelStyle}>
          {typeof label === "string" ? label : renderCustomLabel(label, view)}
        </Text>
      ) : null}
      {chevron ? (
        <View testID="plugin-button-chevron">
          <ChevronDown size={12} color={props.theme.colors.foregroundMuted} />
        </View>
      ) : null}
    </>
  );
  const accessibilityState = useMemo(
    () => ({ busy: entry.pending, disabled }),
    [entry.pending, disabled],
  );
  const trigger = expanded ? (
    <MenuTrigger
      accessibilityRole="button"
      accessibilityLabel={button.title}
      accessibilityState={accessibilityState}
      disabled={disabled}
      style={buttonStyle}
    >
      {contents}
    </MenuTrigger>
  ) : (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={button.title}
      accessibilityState={accessibilityState}
      disabled={disabled}
      onPress={press}
      style={buttonStyle}
    >
      {contents}
    </Pressable>
  );
  const pages = useMemo(() => buttonPages(view, button.behavior), [view, button.behavior]);
  return (
    <MenuRoot compactMode="sheet" open={open} onOpenChange={setOpen}>
      <Tooltip enabledOnMobile={false}>
        <TooltipTrigger asChild>{trigger}</TooltipTrigger>
        <TooltipContent>
          <Text style={styles.tooltipLabel}>{button.title}</Text>
        </TooltipContent>
      </Tooltip>
      {expanded ? (
        <MenuSurface
          sheetTitle={button.title}
          side={composer ? "top" : "bottom"}
          align={composer ? "start" : "end"}
          offset={composer ? 12 : 4}
          minWidth={280}
          maxWidth={420}
          maxHeight={440}
          scrollable
          pages={pages}
        >
          <ButtonSurfaceBody view={view} behavior={button.behavior} path={ROOT_PATH} />
        </MenuSurface>
      ) : null}
    </MenuRoot>
  );
}

function BrokenButton({
  title,
  error,
  compact,
}: {
  title: string;
  error: string;
  compact: boolean;
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        accessibilityRole="button"
        accessibilityLabel={title}
        disabled
        style={headerButtonStyle(compact, {}, true)}
      >
        <ThemedErrorIcon size={16} uniProps={errorIconMapping} />
      </TooltipTrigger>
      <TooltipContent>
        <Text style={styles.tooltipLabel}>{error}</Text>
      </TooltipContent>
    </Tooltip>
  );
}

function createButtonView({
  entry,
  client,
  toast,
  compact,
  hostLabel,
  theme,
}: {
  entry: RegisteredPluginButton;
  client: ReturnType<typeof useHostRuntimeClient>;
  toast: ReturnType<typeof useToast>;
  compact: boolean;
  hostLabel: string;
  theme: PluginTheme;
}): ButtonView | null {
  if (!client) return null;
  return {
    entry,
    client,
    toast,
    state: createPluginClientStateSource(entry.installation.serverId),
    props: {
      ...entry.context,
      theme,
      host: { id: entry.installation.serverId, label: hostLabel },
      layout: { compact, platform: resolvePlatform() },
    },
  };
}

function PluginButtonHost({
  entry,
  compact,
  hostLabel,
  theme,
}: {
  entry: RegisteredPluginButton;
  compact: boolean;
  hostLabel: string;
  theme: PluginTheme;
}) {
  const client = useHostRuntimeClient(entry.installation.serverId);
  const toast = useToast();
  const view = useMemo(
    () => createButtonView({ entry, client, toast, compact, hostLabel, theme }),
    [entry, client, toast, compact, hostLabel, theme],
  );
  const renderError = useCallback(
    (error: string) => <BrokenButton title={entry.button.title} error={error} compact={compact} />,
    [entry.button.title, compact],
  );
  if (!view) return null;
  return (
    <SurfaceErrorBoundary
      installation={entry.installation}
      Surface={entry.button.icon}
      renderError={renderError}
    >
      <ButtonEnvironment view={view}>
        <ButtonControl view={view} />
      </ButtonEnvironment>
    </SurfaceErrorBoundary>
  );
}

const ThemedPluginButton = withUnistyles(PluginButtonHost);

// Overflow rows keep their own plugin runtime; a menu can contain several installations.
function OverflowButton({ view }: { view: ButtonView }) {
  const { button } = view.entry;
  return (
    <ButtonMenuItem
      view={view}
      title={button.title}
      icon={button.icon}
      behavior={button.behavior}
      disabled={button.disabled || view.entry.pending}
      path={ROOT_PATH}
      pageId={buttonPageId(view, [])}
    />
  );
}

function OverflowPages({
  entries,
  compact,
  hostLabel,
  theme,
}: {
  entries: readonly RegisteredPluginButton[];
  compact: boolean;
  hostLabel: string;
  theme: PluginTheme;
}) {
  // The header belongs to one host, but each button keeps its installation's query cache and RPCs.
  const client = useHostRuntimeClient(entries[0].installation.serverId);
  const toast = useToast();
  const menuContent = useMemo(() => {
    const pages: MenuPageDefinition[] = [];
    const rows: ReactNode[] = [];
    for (const entry of entries) {
      const view = createButtonView({ entry, client, toast, compact, hostLabel, theme });
      if (!view) continue;
      rows.push(
        <SurfaceErrorBoundary
          key={entry.key}
          installation={entry.installation}
          Surface={entry.button.icon}
        >
          <ButtonEnvironment view={view}>
            <OverflowButton view={view} />
          </ButtonEnvironment>
        </SurfaceErrorBoundary>,
      );
      if (entry.button.behavior.kind === "action") continue;
      pages.push(
        {
          id: buttonPageId(view, []),
          title: entry.button.title,
          hoverIntent: false,
          content: (
            <ButtonSurfaceBody view={view} behavior={entry.button.behavior} path={ROOT_PATH} />
          ),
        },
        ...buttonPages(view, entry.button.behavior),
      );
    }
    return { pages, rows };
  }, [entries, client, toast, theme, hostLabel, compact]);
  const { t } = useTranslation();
  return (
    <MenuSurface
      sheetTitle={t("workspace.git.actions.moreActions")}
      align="end"
      minWidth={280}
      maxWidth={420}
      maxHeight={440}
      scrollable
      pages={menuContent.pages}
    >
      {menuContent.rows}
    </MenuSurface>
  );
}

const ThemedOverflowPages = withUnistyles(OverflowPages);

function useButtons(
  serverId: string,
  workspaceId: string,
  agentId: string | null,
  slot: ComposerPillSlot = "track",
) {
  const entries = useSyncExternalStore(
    pluginButtonStore.subscribe,
    pluginButtonStore.getSnapshot,
    pluginButtonStore.getSnapshot,
  );
  const compact = useIsCompactFormFactor();
  return useMemo(
    () =>
      entries
        .filter(
          (entry) =>
            buttonMatches(entry, serverId, workspaceId, agentId) &&
            buttonSlot(entry, compact) === slot,
        )
        .map((entry) => resolveButtonForContext(entry, workspaceId, agentId)),
    [entries, serverId, workspaceId, agentId, compact, slot],
  );
}

export function useHasPluginComposerPills(
  serverId: string,
  workspaceId: string,
  agentId: string,
): boolean {
  return useButtons(serverId, workspaceId, agentId).length > 0;
}

/** `toolbar` selects the pills that sit beside the model selector instead of the track bar. */
export function PluginComposerPills({
  serverId,
  workspaceId,
  agentId,
  compact,
  toolbar = false,
}: {
  serverId: string;
  workspaceId: string;
  agentId: string;
  compact: boolean;
  toolbar?: boolean;
}) {
  const entries = useButtons(serverId, workspaceId, agentId, toolbar ? "toolbar" : "track");
  const hosts = useHosts();
  const hostLabel = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  return entries.map((entry) => (
    <ThemedPluginButton
      key={entry.key}
      entry={entry}
      compact={compact}
      hostLabel={hostLabel}
      uniProps={pluginThemeMapping}
    />
  ));
}

/**
 * The pills stacked at the top-right corner of an agent's pane, in registration order. Compact
 * layouts render nothing here; those pills stay in the track bar.
 */
export function PluginComposerCornerPills({
  serverId,
  workspaceId,
  agentId,
}: {
  serverId: string;
  workspaceId: string;
  agentId: string;
}) {
  const entries = useButtons(serverId, workspaceId, agentId, "corner");
  const hosts = useHosts();
  if (!entries.length) return null;
  const hostLabel = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  return (
    <View style={styles.cornerPills} pointerEvents="box-none" testID="plugin-composer-corner-pills">
      {entries.map((entry) => (
        <ThemedPluginButton
          key={entry.key}
          entry={entry}
          compact={false}
          hostLabel={hostLabel}
          uniProps={pluginThemeMapping}
        />
      ))}
    </View>
  );
}

export function PluginDraftComposerPills({
  serverId,
  draft,
  compact,
  hidden = false,
  toolbar = false,
}: {
  serverId: string;
  draft: PluginComposerDraftState;
  compact: boolean;
  hidden?: boolean;
  toolbar?: boolean;
}) {
  const entries = useSyncExternalStore(pluginButtonStore.subscribe, pluginButtonStore.getSnapshot);
  const hosts = useHosts();
  const hostLabel = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  const pills = useMemo(
    () =>
      entries
        .filter(
          (entry) =>
            entry.installation.serverId === serverId &&
            entry.placement === "composer" &&
            entry.showOnDraft &&
            entry.button.visible &&
            buttonInToolbar(entry, compact) === toolbar &&
            entry.context.context === "agent" &&
            !entry.context.workspaceId &&
            !entry.context.agentId,
        )
        // Each composer needs its own context; keep the shared registration unchanged.
        // oxlint-disable-next-line no-map-spread
        .map(
          (entry): RegisteredPluginButton => ({
            ...entry,
            context: {
              context: "draft",
              workspaceId: "",
              draft: draft.forPlugin(entry.installation.id),
            },
            open: entry.open && entry.openContextKey === draft.id,
          }),
        ),
    [entries, serverId, draft, compact, toolbar],
  );
  if (hidden || !pills.length) return null;
  const buttons = pills.map((entry) => (
    <ThemedPluginButton
      key={`${entry.key}:${draft.id}`}
      entry={entry}
      compact={compact}
      hostLabel={hostLabel}
      uniProps={pluginThemeMapping}
    />
  ));
  return toolbar ? buttons : <View style={styles.draftPills}>{buttons}</View>;
}

/** The pills beside the model selector, for an agent composer or a New workspace draft. */
export function PluginComposerToolbarPills({
  serverId,
  workspaceId,
  agentId,
  draft,
}: {
  serverId: string;
  workspaceId?: string | null;
  agentId?: string;
  draft?: PluginComposerDraftState;
}) {
  if (useIsCompactFormFactor()) return null;
  if (draft)
    return <PluginDraftComposerPills serverId={serverId} draft={draft} compact={false} toolbar />;
  if (!agentId) return null;
  return (
    <PluginComposerPills
      serverId={serverId}
      workspaceId={workspaceId ?? ""}
      agentId={agentId}
      compact={false}
      toolbar
    />
  );
}

export function PluginHeaderButtons({
  serverId,
  workspaceId,
}: {
  serverId: string;
  workspaceId: string;
}) {
  const entries = useButtons(serverId, workspaceId, null);
  const compact = useIsCompactFormFactor();
  const { width } = useWindowDimensions();
  const hosts = useHosts();
  const { t } = useTranslation();
  const hostLabel = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  const limit = compact || width < 1100 ? 1 : 3;
  const visible = entries.slice(0, limit);
  const overflow = entries.slice(limit);
  const overflowStyle = useCallback(
    (state: IconButtonChromeState) => headerButtonStyle(compact, state),
    [compact],
  );
  if (entries.length === 0) return null;
  return (
    <View
      accessibilityRole="toolbar"
      accessibilityLabel={t("workspace.header.actions.workspaceActions")}
      style={styles.headerButtons}
    >
      {visible.map((entry) => (
        <ThemedPluginButton
          key={entry.key}
          entry={entry}
          compact={compact}
          hostLabel={hostLabel}
          uniProps={pluginThemeMapping}
        />
      ))}
      {overflow.length > 0 ? (
        <MenuRoot compactMode="sheet">
          <MenuTrigger
            accessibilityRole="button"
            accessibilityLabel={t("workspace.git.actions.moreActions")}
            style={overflowStyle}
          >
            <ThemedMoreIcon size={16} uniProps={mutedIconMapping} />
          </MenuTrigger>
          <ThemedOverflowPages
            entries={overflow}
            compact={compact}
            hostLabel={hostLabel}
            uniProps={pluginThemeMapping}
          />
        </MenuRoot>
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create((theme) => ({
  // The inset matches the composer's own horizontal padding, so the pills line up with its border.
  draftPills: {
    flexDirection: "row",
    gap: theme.spacing[1],
    paddingHorizontal: theme.spacing[4],
    paddingBottom: theme.spacing[2],
  },
  cornerPills: {
    position: "absolute",
    top: theme.spacing[3],
    right: theme.spacing[3],
    zIndex: 10,
    alignItems: "flex-end",
    gap: theme.spacing[2],
  },
  headerButtons: {
    flexDirection: "row",
    alignItems: "center",
    gap: { xs: theme.spacing[1], md: theme.spacing[2] },
  },
  headerButton: {
    height: { xs: 32, md: HEADER_CONTROL_HEIGHT },
    minWidth: { xs: 32, md: HEADER_CONTROL_HEIGHT },
    paddingHorizontal: theme.spacing[2],
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "center",
    gap: theme.spacing[1],
    borderRadius: theme.borderRadius.md,
    borderWidth: theme.borderWidth[1],
    borderColor: theme.colors.borderAccent,
  },
  button: { flexShrink: 1, minWidth: 0, maxWidth: 160 },
  // The corner has no neighbours to make room for, so a label keeps more of its width.
  cornerButton: { maxWidth: 240 },
  // Matches the agent controls' badges so plugin pills read as part of the same toolbar. The pill
  // keeps its width; the agent controls beside it already collapse to fit what is left.
  toolbarButton: {
    height: 28,
    flexShrink: 0,
    maxWidth: 240,
    flexDirection: "row",
    alignItems: "center",
    gap: theme.spacing[1],
    paddingHorizontal: theme.spacing[2],
    marginRight: theme.spacing[1],
    borderRadius: theme.borderRadius["2xl"],
  },
  toolbarLabel: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.foregroundMuted,
    flexShrink: 1,
  },
  active: { backgroundColor: theme.colors.surface2 },
  disabled: { opacity: theme.opacity[50] },
  tooltipLabel: { fontSize: theme.fontSize.sm, color: theme.colors.foreground },
  composerLabel: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.foregroundMuted,
    flexShrink: 1,
  },
  label: { fontSize: theme.fontSize.sm, color: theme.colors.foregroundMuted, flexShrink: 1 },
  icon: {
    width: 16,
    height: 16,
    alignItems: "center",
    justifyContent: "center",
    flexShrink: 0,
    overflow: "hidden",
  },
  content: { padding: theme.spacing[3], gap: theme.spacing[2] },
}));
