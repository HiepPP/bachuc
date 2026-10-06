import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
} from "react";
import { createPortal } from "react-dom";
import type { PluginAssistantSelectionActionContribution } from "@getpaseo/plugin/client";
import { Pressable, Text, View } from "react-native";
import { StyleSheet } from "react-native-unistyles";
import { getOverlayRoot, useOverlayLayer } from "@/lib/overlay-root";
import { useInstalledPlugins } from "@/plugins/registry";
import { createAssistantSelectionClipboardContent } from "./content.web";
import type { AssistantSelectionActionsProps } from "./selection-actions";

interface SelectedText {
  text: string;
  messageId?: string;
}

interface ToolbarPosition {
  top: number;
  left: number;
}

const TOOLBAR_GAP = 8;
const HIDDEN_ANCHOR: CSSProperties = { display: "none" };

function selectionMessageId(range: Range): string | undefined {
  const start = range.startContainer;
  const element = start instanceof Element ? start : start.parentElement;
  return element?.closest("[data-message-id]")?.getAttribute("data-message-id") ?? undefined;
}

/** The current selection's range, when it sits inside `container`. */
function rangeInside(container: Element | null | undefined): Range | null {
  const selection = window.getSelection();
  if (!container || !selection || selection.rangeCount !== 1 || selection.isCollapsed) return null;
  const range = selection.getRangeAt(0);
  return container.contains(range.commonAncestorContainer) ? range : null;
}

/** Above the selection, or nothing once the selection scrolls out of the pane. */
function toolbarPosition(range: Range, container: Element): ToolbarPosition | null {
  const rect = range.getBoundingClientRect();
  const bounds = container.getBoundingClientRect();
  if (rect.bottom < bounds.top || rect.top > bounds.bottom) return null;
  return {
    top: Math.max(bounds.top, rect.top) - TOOLBAR_GAP,
    left: rect.left + rect.width / 2,
  };
}

/**
 * Shows plugin actions over text selected in this pane's assistant messages. A hidden anchor
 * finds the pane's container, so a selection in another pane never shows this toolbar.
 */
export function AssistantSelectionActions({
  serverId,
  agentId,
  workspaceId,
}: AssistantSelectionActionsProps) {
  const plugins = useInstalledPlugins();
  const actions = useMemo(
    () =>
      plugins
        .filter((plugin) => plugin.serverId === serverId)
        .flatMap((plugin) => plugin.assistantSelectionActions ?? []),
    [plugins, serverId],
  );
  const anchorRef = useRef<HTMLSpanElement | null>(null);
  const [selected, setSelected] = useState<SelectedText | null>(null);
  const [position, setPosition] = useState<ToolbarPosition | null>(null);
  const layer = useOverlayLayer("floating");
  const hasActions = actions.length > 0;

  useEffect(() => {
    if (!hasActions) {
      setSelected(null);
      setPosition(null);
      return;
    }
    const container = () => anchorRef.current?.parentElement;
    // Scroll and resize only move the toolbar; the Markdown is built once per selection change.
    const place = () => {
      const pane = container();
      const range = rangeInside(pane);
      setPosition(pane && range ? toolbarPosition(range, pane) : null);
    };
    const select = () => {
      const range = rangeInside(container());
      const content = range
        ? createAssistantSelectionClipboardContent(window.getSelection())
        : null;
      setSelected(
        range && content ? { text: content.plainText, messageId: selectionMessageId(range) } : null,
      );
      place();
    };
    document.addEventListener("selectionchange", select);
    window.addEventListener("scroll", place, true);
    window.addEventListener("resize", place);
    return () => {
      document.removeEventListener("selectionchange", select);
      window.removeEventListener("scroll", place, true);
      window.removeEventListener("resize", place);
    };
  }, [hasActions]);

  // Keeps the selection alive while the pointer presses a toolbar button.
  const keepSelection = useCallback((event: MouseEvent<HTMLDivElement>) => {
    event.preventDefault();
  }, []);

  const runAction = useCallback(
    (action: PluginAssistantSelectionActionContribution) => {
      if (!selected) return;
      const selection = {
        serverId,
        workspaceId: workspaceId ?? null,
        agentId,
        ...(selected.messageId ? { messageId: selected.messageId } : {}),
        text: selected.text,
      };
      window.getSelection()?.removeAllRanges();
      setSelected(null);
      setPosition(null);
      void Promise.resolve(action.onSelect(selection)).catch((error: unknown) => {
        console.error(`[plugin] selection action ${action.id} failed`, error);
      });
    },
    [agentId, serverId, selected, workspaceId],
  );

  const toolbarStyle = useMemo<CSSProperties | null>(
    () =>
      position
        ? {
            position: "fixed",
            top: position.top,
            left: position.left,
            transform: "translate(-50%, -100%)",
            zIndex: layer,
            pointerEvents: "auto",
          }
        : null,
    [layer, position],
  );

  return (
    <>
      <span ref={anchorRef} style={HIDDEN_ANCHOR} />
      {selected && toolbarStyle
        ? createPortal(
            <div onMouseDown={keepSelection} style={toolbarStyle}>
              <View style={styles.toolbar} testID="assistant-selection-actions">
                {actions.map((action) => (
                  <SelectionActionButton key={action.id} action={action} onRun={runAction} />
                ))}
              </View>
            </div>,
            getOverlayRoot(),
          )
        : null}
    </>
  );
}

interface SelectionActionButtonProps {
  action: PluginAssistantSelectionActionContribution;
  onRun: (action: PluginAssistantSelectionActionContribution) => void;
}

function SelectionActionButton({ action, onRun }: SelectionActionButtonProps) {
  const handlePress = useCallback(() => onRun(action), [action, onRun]);
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={action.title}
      style={styles.action}
      onPress={handlePress}
    >
      <Text style={styles.actionText}>{action.title}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create((theme) => ({
  toolbar: {
    flexDirection: "row",
    gap: theme.spacing[1],
    padding: theme.spacing[1],
    borderRadius: theme.borderRadius.lg,
    backgroundColor: theme.colors.popover,
    borderWidth: theme.borderWidth[1],
    borderColor: theme.colors.borderAccent,
    ...theme.shadow.md,
  },
  action: {
    paddingVertical: theme.spacing[1],
    paddingHorizontal: theme.spacing[2],
    borderRadius: theme.borderRadius.md,
  },
  actionText: {
    color: theme.colors.foreground,
    fontSize: theme.fontSize.sm,
  },
}));
