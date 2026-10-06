import type { InstalledPlugin } from "../types";
import type { PluginResourceComposerAttachment } from "./model";

/**
 * Hands a chip press to the `onOpen` of the attachment source that made the chip. Returns false
 * when that plugin is not installed on the host or its source has no `onOpen`.
 */
export function openPluginResourceAttachment(
  plugins: readonly InstalledPlugin[],
  serverId: string,
  attachment: PluginResourceComposerAttachment,
): boolean {
  const onOpen = plugins
    .find((plugin) => plugin.serverId === serverId && plugin.id === attachment.pluginId)
    ?.attachmentSources.find((source) => source.id === attachment.sourceId)?.onOpen;
  if (!onOpen) return false;
  void (async () => {
    await onOpen(attachment.item);
  })().catch((error: unknown) => {
    console.error(`[plugin:${attachment.pluginId}] could not open a chip`, error);
  });
  return true;
}
