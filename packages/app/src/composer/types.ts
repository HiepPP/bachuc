import type { PluginComposerSendKey } from "@getpaseo/plugin/client";
import type { AttachmentMetadata, ComposerAttachment } from "@/attachments/types";

export type ImageAttachment = AttachmentMetadata;

export interface MessagePayload {
  text: string;
  attachments: ComposerAttachment[];
  cwd: string;
  forceSend?: boolean;
  /** The key that sent the message, for plugin composer interceptors. */
  sendKey?: PluginComposerSendKey;
}

export interface TextReplacement {
  key: string;
  text: string;
}
