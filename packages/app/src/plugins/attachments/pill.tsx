import { useCallback, useMemo } from "react";
import { View } from "react-native";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import { AttachmentLabel, AttachmentPill } from "@/components/attachment-pill";
import { EditingTextInput } from "@/components/ui/text-input";
import type { ComposerAttachment } from "@/attachments/types";
import { ICON_SIZE, type Theme } from "@/styles/theme";
import { resolvePluginIcon } from "../icons";
import type { PluginResourceComposerAttachment } from "./model";

const iconColorMapping = (theme: Theme) => ({ color: theme.colors.foregroundMuted });

function ResourceIcon({
  Icon,
  color = "",
}: {
  Icon: ReturnType<typeof resolvePluginIcon>;
  color?: string;
}) {
  return <Icon size={ICON_SIZE.sm} color={color} />;
}

const ThemedResourceIcon = withUnistyles(ResourceIcon);

interface PluginResourceAttachmentPillProps {
  attachment: PluginResourceComposerAttachment;
  index: number;
  disabled: boolean;
  onOpen: (attachment: ComposerAttachment) => void;
  onRemove: (index: number) => void;
  openLabel: (kind: string, identifier: string) => string;
  removeLabel: (kind: string, identifier: string) => string;
  /** Shown only for a chip that takes a comment. */
  onCommentChange?: (index: number, comment: string) => void;
  commentPlaceholder?: string;
}

export function PluginResourceAttachmentPill({
  attachment,
  index,
  disabled,
  onOpen,
  onRemove,
  openLabel,
  removeLabel,
  onCommentChange,
  commentPlaceholder,
}: PluginResourceAttachmentPillProps) {
  const Icon = resolvePluginIcon(attachment.sourceIcon);
  const handleOpen = useCallback(() => onOpen(attachment), [attachment, onOpen]);
  const handleRemove = useCallback(() => onRemove(index), [index, onRemove]);
  const handleCommentChange = useCallback(
    (comment: string) => onCommentChange?.(index, comment),
    [index, onCommentChange],
  );
  const icon = useMemo(
    () => <ThemedResourceIcon Icon={Icon} uniProps={iconColorMapping} />,
    [Icon],
  );
  const pill = (
    <AttachmentPill
      testID="composer-plugin-resource-attachment-pill"
      onOpen={handleOpen}
      onRemove={handleRemove}
      openAccessibilityLabel={openLabel(attachment.sourceTitle, attachment.item.identifier)}
      removeAccessibilityLabel={removeLabel(attachment.sourceTitle, attachment.item.identifier)}
      disabled={disabled}
    >
      <AttachmentLabel
        icon={icon}
        title={attachment.item.title}
        subtitle={`${attachment.sourceTitle} ${attachment.item.identifier}`}
      />
    </AttachmentPill>
  );
  if (!attachment.commentable || !onCommentChange) return pill;
  return (
    <View style={styles.withComment}>
      {pill}
      <EditingTextInput
        testID="composer-plugin-resource-attachment-comment"
        style={styles.comment}
        initialValue={attachment.comment ?? ""}
        onChangeText={handleCommentChange}
        placeholder={commentPlaceholder}
        accessibilityLabel={commentPlaceholder}
        editable={!disabled}
        multiline
      />
    </View>
  );
}

const styles = StyleSheet.create((theme) => ({
  withComment: {
    gap: theme.spacing[1],
    maxWidth: 320,
  },
  comment: {
    minHeight: 32,
    paddingVertical: theme.spacing[1],
    paddingHorizontal: theme.spacing[2],
    borderRadius: theme.borderRadius.md,
    borderWidth: theme.borderWidth[1],
    borderColor: theme.colors.border,
    color: theme.colors.foreground,
    fontSize: theme.fontSize.sm,
  },
}));
