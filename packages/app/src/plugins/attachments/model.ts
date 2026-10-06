import { z } from "zod";
import type { AgentAttachment } from "@getpaseo/protocol/messages";
import { PluginAttachmentItemSchema, type PluginAttachmentItem } from "@getpaseo/plugin";
import type { UserComposerAttachment } from "@/attachments/types";

export const PluginResourceComposerAttachmentSchema = z.object({
  kind: z.literal("plugin_resource"),
  pluginId: z.string().min(1),
  sourceId: z.string().min(1),
  sourceTitle: z.string().min(1),
  sourceIcon: z.string().min(1),
  item: PluginAttachmentItemSchema,
  /** The chip offers a comment field; set by plugins that add chips from code. */
  commentable: z.boolean().optional(),
  comment: z.string().optional(),
});

export type PluginResourceComposerAttachment = z.infer<
  typeof PluginResourceComposerAttachmentSchema
>;

export interface PluginResourceSourceIdentity {
  pluginId: string;
  sourceId: string;
  sourceTitle: string;
  sourceIcon: string;
}

export function createPluginResourceAttachment(
  source: PluginResourceSourceIdentity,
  item: PluginAttachmentItem,
): PluginResourceComposerAttachment {
  return {
    kind: "plugin_resource",
    ...source,
    item,
  };
}

function samePluginResource(attachment: PluginResourceComposerAttachment) {
  return (candidate: UserComposerAttachment) =>
    candidate.kind === "plugin_resource" &&
    candidate.pluginId === attachment.pluginId &&
    candidate.sourceId === attachment.sourceId &&
    candidate.item.id === attachment.item.id;
}

export function togglePluginResourceAttachment(
  current: UserComposerAttachment[],
  attachment: PluginResourceComposerAttachment,
): UserComposerAttachment[] {
  const matches = samePluginResource(attachment);
  if (current.some(matches)) {
    return current.filter((candidate) => !matches(candidate));
  }
  return [...current, attachment];
}

/**
 * Adds the chip, or replaces the one with the same plugin, source, and item in place. A replaced
 * chip keeps the user's comment, because the comment field shows its first value until remount.
 */
export function upsertPluginResourceAttachment(
  current: UserComposerAttachment[],
  attachment: PluginResourceComposerAttachment,
): UserComposerAttachment[] {
  const matches = samePluginResource(attachment);
  if (!current.some(matches)) return [...current, attachment];
  return current.map((candidate) => {
    if (candidate.kind !== "plugin_resource" || !matches(candidate)) return candidate;
    const comment = attachment.comment ?? candidate.comment;
    return comment === undefined ? attachment : { ...attachment, comment };
  });
}

export function pluginResourceAttachmentToAgentAttachment(
  attachment: PluginResourceComposerAttachment,
): AgentAttachment {
  const comment = attachment.commentable ? attachment.comment?.trim() : undefined;
  return {
    type: "text",
    mimeType: "text/plain",
    title: `${attachment.item.identifier} ${attachment.item.title}`,
    text: comment ? `${attachment.item.text}\n\nComment: ${comment}` : attachment.item.text,
    externalResource: {
      provider: attachment.pluginId,
      providerLabel: attachment.sourceTitle,
      resourceType: attachment.item.resourceType,
      id: attachment.item.id,
      identifier: attachment.item.identifier,
      title: attachment.item.title,
      url: attachment.item.url,
    },
  };
}
