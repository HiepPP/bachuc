import type { AgentMessageQueueSummary } from "@getpaseo/protocol/messages";
import type {
  AttachmentMetadata,
  ComposerAttachment,
  UserComposerAttachment,
} from "@/attachments/types";
import {
  splitComposerAttachmentsForSubmit,
  type ComposerAttachmentSubmitFormat,
} from "@/composer/attachments/submit";
import type { QueuedComposerMessage } from "@/composer/actions";

// COMPAT(agentMessageQueue): the daemon owns the queue on hosts with
// server_info.features.agentMessageQueue. Older hosts keep the client-side queue.

export interface DaemonQueueClient {
  enqueueAgentMessage: (
    agentId: string,
    input: {
      text: string;
      images?: Array<{ data: string; mimeType: string }>;
      attachments?: ReturnType<typeof splitComposerAttachmentsForSubmit>["attachments"];
    },
  ) => Promise<unknown>;
  cancelQueuedAgentMessage: (
    agentId: string,
    itemId: string,
  ) => Promise<{
    item: {
      text: string;
      images?: Array<{ data: string; mimeType: string }>;
    } | null;
  }>;
}

/** Rows for the queued list. The row shows text only, so attachments stay out. */
export function queuedMessagesFromDaemon(
  queue: AgentMessageQueueSummary | undefined,
): QueuedComposerMessage[] {
  return (queue?.items ?? []).map((item) => ({ id: item.id, text: item.text, attachments: [] }));
}

/**
 * Edit loads a message back into the composer. Only text and images round-trip, so a
 * message with any other attachment stays in the queue rather than lose it.
 */
export function canEditDaemonQueuedMessage(
  queue: AgentMessageQueueSummary | undefined,
  itemId: string,
): boolean {
  const item = queue?.items.find((candidate) => candidate.id === itemId);
  return Boolean(item?.attachmentKinds.every((kind) => kind === "image"));
}

/** The `beforeItemId` that moves an item one step, or null when it cannot move. */
export function daemonQueueMoveTarget(
  items: ReadonlyArray<{ id: string }>,
  itemId: string,
  direction: "up" | "down",
): { beforeItemId: string | null } | null {
  const index = items.findIndex((item) => item.id === itemId);
  if (index === -1) return null;
  if (direction === "up") {
    const previous = items[index - 1];
    return previous ? { beforeItemId: previous.id } : null;
  }
  if (index >= items.length - 1) return null;
  return { beforeItemId: items[index + 2]?.id ?? null };
}

export interface EnqueueDaemonComposerMessageInput {
  client: DaemonQueueClient;
  agentId: string;
  text: string;
  attachments: ComposerAttachment[];
  attachmentSubmitFormat?: ComposerAttachmentSubmitFormat;
  encodeImages: (
    images: AttachmentMetadata[],
  ) => Promise<Array<{ data: string; mimeType: string }> | undefined>;
}

/** Returns false when there is nothing to queue. */
export async function enqueueDaemonComposerMessage(
  input: EnqueueDaemonComposerMessageInput,
): Promise<boolean> {
  const text = input.text.trim();
  if (!text && input.attachments.length === 0) return false;
  const wirePayload = splitComposerAttachmentsForSubmit(input.attachments, {
    format: input.attachmentSubmitFormat,
  });
  const images = await input.encodeImages(wirePayload.images);
  await input.client.enqueueAgentMessage(input.agentId, {
    text,
    ...(images && images.length > 0 ? { images } : {}),
    ...(wirePayload.attachments.length > 0 ? { attachments: wirePayload.attachments } : {}),
  });
  return true;
}

export interface TakeBackDaemonQueuedMessageInput {
  client: DaemonQueueClient;
  agentId: string;
  itemId: string;
  persistFromDataUrl: (input: {
    dataUrl: string;
    mimeType: string;
    fileName: string | null;
  }) => Promise<AttachmentMetadata>;
}

export interface TakenBackDaemonQueuedMessage {
  text: string;
  attachments: UserComposerAttachment[];
  /** Images the device could not save. The text always comes back. */
  failedImageCount: number;
}

/** Removes the message from the daemon queue and returns it as composer content. */
export async function takeBackDaemonQueuedMessage(
  input: TakeBackDaemonQueuedMessageInput,
): Promise<TakenBackDaemonQueuedMessage | null> {
  const { item } = await input.client.cancelQueuedAgentMessage(input.agentId, input.itemId);
  if (!item) return null;
  // The daemon already removed the item, so one failed image must not lose the rest.
  const results = await Promise.allSettled(
    (item.images ?? []).map((image) =>
      input.persistFromDataUrl({
        dataUrl: `data:${image.mimeType};base64,${image.data}`,
        mimeType: image.mimeType,
        fileName: null,
      }),
    ),
  );
  const attachments: UserComposerAttachment[] = [];
  for (const result of results) {
    if (result.status === "fulfilled") {
      attachments.push({ kind: "image", metadata: result.value });
    }
  }
  return { text: item.text, attachments, failedImageCount: results.length - attachments.length };
}
