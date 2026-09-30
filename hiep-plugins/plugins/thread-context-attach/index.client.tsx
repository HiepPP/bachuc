import type { PluginClientContext } from "@getpaseo/plugin/client";
import { threadAttachments } from "./shared/threads";

export default function contribute(client: PluginClientContext) {
  const removeAttachments = client.addAttachmentSource(threadAttachments);
  return () => {
    removeAttachments();
  };
}
