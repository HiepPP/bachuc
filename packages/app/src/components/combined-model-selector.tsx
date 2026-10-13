import { useCallback, useEffect, useMemo, useRef, useState, type Ref, type RefObject } from "react";
import { Pressable, Text, View, type PressableStateCallbackType } from "react-native";
import { useTranslation } from "react-i18next";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import type { AgentProvider } from "@getpaseo/protocol/agent-types";
import type { AgentProfilePicker, AgentProfileSeed } from "@/agent-profiles";
import { ComboboxTrigger } from "@/components/ui/combobox-trigger";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { Combobox, type ComboboxOption, type ComboboxProps } from "@/components/ui/combobox";
import { ModelBrowser, ModelProviderGlyph, useModelBrowser } from "@/components/model-browser";
import { resolveModelBrowserScrolling } from "@/components/model-browser-view";
import { useIsCompactFormFactor } from "@/constants/layout";
import { isNative, isWeb } from "@/constants/platform";
import {
  composeViewRefs,
  useWheelSelect,
  type WheelSelect,
} from "@/composer/agent-controls/use-wheel-select";
import { useWheelListener } from "@/hooks/use-wheel-listener";
import {
  getProviderModelRows,
  type ProviderSelectionModelRow,
  type ProviderSelectorProvider,
} from "@/provider-selection/provider-selection";
import { ICON_SIZE, type Theme } from "@/styles/theme";

const EMPTY_COMBOBOX_OPTIONS: ComboboxOption[] = [];
const ThemedLoadingSpinner = withUnistyles(LoadingSpinner);

const foregroundMutedMapping = (theme: Theme) => ({
  color: theme.colors.foregroundMuted,
});

function noop() {}

function getCurrentProviderRows(
  providers: ProviderSelectorProvider[],
  selectedProvider: string,
): ProviderSelectionModelRow[] {
  const providerId = selectedProvider.trim();
  const provider = providers.find((entry) => entry.id === providerId);
  return provider ? getProviderModelRows(provider) : [];
}

/** Wheel options for the model pill: the current provider's row keys, in row order (Q-028). */
export function resolveCurrentProviderModelKeys(
  providers: ProviderSelectorProvider[],
  selectedProvider: string,
): string[] {
  return getCurrentProviderRows(providers, selectedProvider).map((row) => row.favoriteKey);
}

/**
 * The row key of the model the pill label shows. Mirrors the fallback of
 * resolveSelectedModelLabel: an unset model resolves to the default row, then the first row.
 * A model id that is not in the list returns null, as the label shows the raw id.
 */
export function resolveCurrentProviderModelKey(input: {
  providers: ProviderSelectorProvider[];
  selectedProvider: string;
  selectedModel: string;
}): string | null {
  const rows = getCurrentProviderRows(input.providers, input.selectedProvider);
  const match = rows.find((row) => row.modelId === input.selectedModel);
  if (match) return match.favoriteKey;
  if (input.selectedModel.trim()) return null;
  return (rows.find((row) => row.isDefault) ?? rows[0])?.favoriteKey ?? null;
}

/**
 * A null valueId is an off-list model id (for example a model no longer in the catalog).
 * The wheel ignores it, so one stray tick cannot replace it; only the popover leaves it.
 */
export function isModelWheelEnabled(input: {
  isComposerToolbar: boolean;
  disabled: boolean;
  isLoading: boolean;
  count: number;
  valueId: string | null;
}): boolean {
  return (
    input.isComposerToolbar &&
    !input.disabled &&
    !input.isLoading &&
    input.count >= 2 &&
    input.valueId !== null
  );
}

export function splitModelRowKey(key: string): { provider: string; modelId: string } | null {
  const separator = key.indexOf(":");
  if (separator <= 0) return null;
  return { provider: key.slice(0, separator), modelId: key.slice(separator + 1) };
}

/**
 * The wheel handler and trigger ref for the ComboboxTrigger branch. Wheel stepping is
 * composer-only: only the composer passes `toolbar` (Q-033), so every other caller keeps the
 * plain anchor ref and gets a null handler.
 */
function useModelWheel(input: {
  anchorRef: RefObject<View | null>;
  isComposerToolbar: boolean;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  disabled: boolean;
  isLoading: boolean;
  providers: ProviderSelectorProvider[];
  selectedProvider: string;
  selectedModel: string;
  onSelect: (provider: AgentProvider, modelId: string) => void;
}): { triggerRef: Ref<View>; wheel: WheelSelect } {
  const { anchorRef, isComposerToolbar, providers, selectedProvider, selectedModel, onSelect } =
    input;
  const optionIds = useMemo(
    () => resolveCurrentProviderModelKeys(providers, selectedProvider),
    [providers, selectedProvider],
  );
  const valueId = useMemo(
    () => resolveCurrentProviderModelKey({ providers, selectedProvider, selectedModel }),
    [providers, selectedModel, selectedProvider],
  );
  const handleWheelSelect = useCallback(
    (key: string) => {
      const parsed = splitModelRowKey(key);
      if (parsed) onSelect(parsed.provider, parsed.modelId);
    },
    [onSelect],
  );
  const wheel = useWheelSelect({
    enabled: isModelWheelEnabled({
      isComposerToolbar,
      disabled: input.disabled,
      isLoading: input.isLoading,
      count: optionIds.length,
      valueId,
    }),
    optionIds,
    valueId,
    onSelect: handleWheelSelect,
    open: input.isOpen,
    onOpen: () => input.onOpenChange(true),
  });
  const wheelRef = useWheelListener(wheel.handler, wheel.onHover);
  const composedRef = useMemo(() => composeViewRefs(anchorRef, wheelRef), [anchorRef, wheelRef]);
  return { triggerRef: isComposerToolbar ? composedRef : anchorRef, wheel };
}

interface CombinedModelSelectorProps {
  providers: ProviderSelectorProvider[];
  selectedProvider: string;
  selectedModel: string;
  onSelect: (provider: AgentProvider, modelId: string) => void;
  isLoading: boolean;
  profiles?: AgentProfilePicker | null;
  onApplyProfile?: (profileId: string) => void;
  onEditProfiles?: () => void;
  onCreateProfile?: (seed: AgentProfileSeed) => void;
  onEditProfile?: (profileId: string) => void;
  renderTrigger?: (input: {
    selectedModelLabel: string;
    onPress: () => void;
    disabled: boolean;
    isOpen: boolean;
    hovered: boolean;
    pressed: boolean;
  }) => React.ReactNode;
  onOpen?: () => void;
  onClose?: () => void;
  onRetryProvider?: (provider: AgentProvider) => void;
  isRetryingProvider?: boolean;
  disabled?: boolean;
  serverId?: string | null;
  desktopPlacement?: ComboboxProps["desktopPlacement"];
  desktopMinWidth?: number;
  /**
   * Render the custom trigger as a full-width form field: the outer Pressable
   * becomes a transparent passthrough that stretches its child edge-to-edge and
   * stops painting its own hover/pressed background and rounded corners. The
   * trigger itself owns the field visuals and reads hovered/pressed to show its
   * active state. Without this the trigger stays a content-width toolbar chip
   * (the composer's layout).
   */
  triggerFill?: boolean;
  toolbar?: {
    glyphSize: number;
    showCaret: boolean;
  };
}

export function CombinedModelSelector({
  providers,
  selectedProvider,
  selectedModel,
  onSelect,
  isLoading,
  profiles = null,
  onApplyProfile,
  onEditProfiles,
  onCreateProfile,
  onEditProfile,
  renderTrigger,
  onOpen,
  onClose,
  onRetryProvider,
  isRetryingProvider = false,
  disabled = false,
  serverId = null,
  desktopPlacement,
  desktopMinWidth,
  triggerFill = false,
  toolbar,
}: CombinedModelSelectorProps) {
  const { t } = useTranslation();
  const isCompact = useIsCompactFormFactor();
  const modelBrowserScrolling = resolveModelBrowserScrolling({ isNative, isCompact });
  const anchorRef = useRef<View>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [isContentReady, setIsContentReady] = useState(isWeb);
  const browser = useModelBrowser({
    providers,
    selectedProvider,
    selectedModel,
    isLoading,
    profiles,
    serverId,
  });
  const { prepareToOpen, reset } = browser;

  const handleOpenChange = useCallback(
    (open: boolean) => {
      setIsOpen(open);
      if (open) {
        prepareToOpen();
        onOpen?.();
        return;
      }
      reset();
      onClose?.();
    },
    [onClose, onOpen, prepareToOpen, reset],
  );
  const { triggerRef: comboboxTriggerRef, wheel } = useModelWheel({
    anchorRef,
    isComposerToolbar: toolbar !== undefined,
    isOpen,
    onOpenChange: handleOpenChange,
    disabled,
    isLoading,
    providers,
    selectedProvider,
    selectedModel,
    onSelect,
  });

  const handleSelect = useCallback(
    (provider: string, modelId: string) => {
      onSelect(provider, modelId);
      handleOpenChange(false);
    },
    [handleOpenChange, onSelect],
  );

  useEffect(() => {
    if (isWeb) return () => {};
    if (!isOpen) {
      setIsContentReady(false);
      return () => {};
    }
    const frame = requestAnimationFrame(() => {
      setIsContentReady(true);
    });
    return () => cancelAnimationFrame(frame);
  }, [isOpen]);

  const handleTriggerPress = useCallback(() => {
    handleOpenChange(!isOpen);
  }, [handleOpenChange, isOpen]);

  const triggerStyle = useCallback(
    ({ pressed, hovered }: PressableStateCallbackType & { hovered?: boolean }) => {
      if (triggerFill) {
        return [
          styles.trigger,
          styles.customTriggerWrapper,
          styles.triggerFill,
          disabled && styles.triggerDisabled,
        ];
      }
      return [
        styles.trigger,
        Boolean(hovered) && styles.triggerHovered,
        (pressed || isOpen) && styles.triggerPressed,
        disabled && styles.triggerDisabled,
        renderTrigger ? styles.customTriggerWrapper : null,
      ];
    },
    [disabled, isOpen, renderTrigger, triggerFill],
  );

  const handleApplyProfile = useCallback(
    (profileId: string) => {
      onApplyProfile?.(profileId);
      handleOpenChange(false);
    },
    [handleOpenChange, onApplyProfile],
  );

  const handleEditProfiles = useCallback(() => {
    handleOpenChange(false);
    onEditProfiles?.();
  }, [handleOpenChange, onEditProfiles]);

  const handleCreateProfile = useCallback(
    (seed: AgentProfileSeed) => {
      handleOpenChange(false);
      onCreateProfile?.(seed);
    },
    [handleOpenChange, onCreateProfile],
  );

  const handleEditProfile = useCallback(
    (profileId: string) => {
      handleOpenChange(false);
      onEditProfile?.(profileId);
    },
    [handleOpenChange, onEditProfile],
  );

  const selectorBody = isContentReady ? (
    <ModelBrowser
      state={browser}
      onSelect={handleSelect}
      onApplyProfile={handleApplyProfile}
      onEditProfiles={onEditProfiles ? handleEditProfiles : undefined}
      onCreateProfile={onCreateProfile ? handleCreateProfile : undefined}
      onEditProfile={onEditProfile ? handleEditProfile : undefined}
      onRetryProvider={onRetryProvider}
      isRetryingProvider={isRetryingProvider}
      scrolling={modelBrowserScrolling}
    />
  ) : (
    <View style={styles.sheetLoadingState}>
      <ThemedLoadingSpinner size={ICON_SIZE.sm} uniProps={foregroundMutedMapping} />
      <Text style={styles.sheetLoadingText}>{t("modelSelector.loadingSelector")}</Text>
    </View>
  );

  return (
    <>
      {renderTrigger ? (
        <Pressable
          ref={anchorRef}
          collapsable={false}
          disabled={disabled}
          onPress={handleTriggerPress}
          style={triggerStyle}
          accessibilityRole="button"
          accessibilityLabel={t("modelSelector.selectedModel", {
            model: browser.selectedModelLabel,
          })}
          testID="combined-model-selector"
        >
          {({ pressed, hovered }: PressableStateCallbackType & { hovered?: boolean }) =>
            renderTrigger({
              selectedModelLabel: browser.triggerLabel,
              onPress: handleTriggerPress,
              disabled,
              isOpen,
              hovered: Boolean(hovered),
              pressed,
            })
          }
        </Pressable>
      ) : (
        <ComboboxTrigger
          ref={comboboxTriggerRef}
          collapsable={false}
          disabled={disabled}
          onPress={handleTriggerPress}
          style={triggerStyle}
          accessibilityRole="button"
          accessibilityLabel={t("modelSelector.selectedModel", {
            model: browser.selectedModelLabel,
          })}
          testID="combined-model-selector"
          chevron={toolbar?.showCaret === false ? null : undefined}
        >
          {selectedProvider.trim().length > 0 ? (
            <View style={resolveToolbarGlyphBoxStyle(toolbar?.glyphSize)}>
              <ModelProviderGlyph
                provider={selectedProvider}
                serverId={serverId}
                size={toolbar?.glyphSize ?? ICON_SIZE.md}
              />
            </View>
          ) : null}
          <Text
            style={toolbar ? styles.toolbarTriggerText : styles.triggerText}
            numberOfLines={1}
            ellipsizeMode="tail"
          >
            {browser.triggerLabel}
          </Text>
        </ComboboxTrigger>
      )}
      <Combobox
        options={EMPTY_COMBOBOX_OPTIONS}
        value=""
        onSelect={noop}
        open={isOpen}
        onOpenChange={handleOpenChange}
        anchorRef={anchorRef}
        desktopPlacement={desktopPlacement}
        desktopMinWidth={desktopMinWidth}
        onBackdropWheel={wheel.handler}
        closeOnPointerLeave={wheel.openedByPointer}
        desktopLockWidth
        desktopFixedHeight={browser.desktopFixedHeight}
        desktopChildrenScrollEnabled={false}
        header={browser.header}
        mobileChildrenScrollEnabled={!browser.isProviderView || !isNative}
        mobileChildrenContentContainerStyle={styles.mobileBrowserContent}
      >
        {selectorBody}
      </Combobox>
    </>
  );
}

function resolveToolbarGlyphBoxStyle(glyphSize: number | undefined) {
  if (glyphSize === 20) return styles.toolbarGlyph20;
  if (glyphSize === 14) return styles.toolbarGlyph14;
  return styles.toolbarGlyph16;
}

const styles = StyleSheet.create((theme) => ({
  mobileBrowserContent: {
    paddingHorizontal: 0,
  },
  trigger: {
    height: 28,
    minWidth: 0,
    flexShrink: 1,
    flexDirection: "row",
    alignItems: "center",
    backgroundColor: "transparent",
    gap: theme.spacing[1],
    paddingHorizontal: theme.spacing[2],
    borderRadius: theme.borderRadius["2xl"],
  },
  triggerHovered: {
    backgroundColor: theme.colors.surface2,
  },
  toolbarGlyph14: {
    width: 14,
    height: 14,
    flexShrink: 0,
  },
  toolbarGlyph16: {
    width: 16,
    height: 16,
    flexShrink: 0,
  },
  toolbarGlyph20: {
    width: 20,
    height: 20,
    flexShrink: 0,
  },
  triggerPressed: {
    backgroundColor: theme.colors.surface0,
  },
  triggerDisabled: {
    opacity: 0.5,
  },
  triggerText: {
    minWidth: 0,
    flexShrink: 1,
    color: theme.colors.foregroundMuted,
    fontSize: theme.fontSize.base,
    fontWeight: theme.fontWeight.normal,
  },
  // Composer toolbar controls are the xs tier: sm labels, the same size as the pills above.
  toolbarTriggerText: {
    minWidth: 0,
    flexShrink: 1,
    color: theme.colors.foregroundMuted,
    fontSize: theme.fontSize.sm,
    fontWeight: theme.fontWeight.normal,
  },
  customTriggerWrapper: {
    paddingHorizontal: 0,
    paddingVertical: 0,
    height: "auto",
  },
  triggerFill: {
    alignSelf: "stretch",
    flexShrink: 0,
    flexDirection: "column",
    alignItems: "stretch",
    backgroundColor: "transparent",
    borderRadius: 0,
  },
  sheetLoadingState: {
    minHeight: 160,
    justifyContent: "center",
    alignItems: "center",
    gap: theme.spacing[2],
  },
  sheetLoadingText: {
    color: theme.colors.foregroundMuted,
    fontSize: theme.fontSize.base,
  },
}));
