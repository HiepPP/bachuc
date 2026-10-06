import { describe, expect, it, vi } from "vitest";
import type { AttachmentMetadata } from "@/attachments/types";
import {
  canEditDaemonQueuedMessage,
  daemonQueueMoveTarget,
  enqueueDaemonComposerMessage,
  queuedMessagesFromDaemon,
  takeBackDaemonQueuedMessage,
  type DaemonQueueClient,
} from "./daemon-queue";

const queue = {
  held: false,
  items: [
    { id: "a", text: "First", attachmentKinds: [], createdAt: "2026-10-07T00:00:00.000Z" },
    { id: "b", text: "Second", attachmentKinds: ["image"], createdAt: "2026-10-07T00:00:01.000Z" },
    { id: "c", text: "Third", attachmentKinds: ["text"], createdAt: "2026-10-07T00:00:02.000Z" },
  ],
};

function createClient(): DaemonQueueClient & {
  enqueueAgentMessage: ReturnType<typeof vi.fn>;
  cancelQueuedAgentMessage: ReturnType<typeof vi.fn>;
} {
  return {
    enqueueAgentMessage: vi.fn(async () => ({})),
    cancelQueuedAgentMessage: vi.fn(async () => ({ item: null })),
  };
}

describe("daemon queue", () => {
  it("shows the daemon items as queued rows, and none without a queue", () => {
    expect(queuedMessagesFromDaemon(queue).map((item) => [item.id, item.text])).toEqual([
      ["a", "First"],
      ["b", "Second"],
      ["c", "Third"],
    ]);
    expect(queuedMessagesFromDaemon(undefined)).toEqual([]);
  });

  it("edits only messages whose attachments round-trip", () => {
    expect(canEditDaemonQueuedMessage(queue, "a")).toBe(true);
    expect(canEditDaemonQueuedMessage(queue, "b")).toBe(true);
    expect(canEditDaemonQueuedMessage(queue, "c")).toBe(false);
    expect(canEditDaemonQueuedMessage(queue, "missing")).toBe(false);
  });

  it("moves an item one step and stops at the ends", () => {
    expect(daemonQueueMoveTarget(queue.items, "a", "up")).toBeNull();
    expect(daemonQueueMoveTarget(queue.items, "b", "up")).toEqual({ beforeItemId: "a" });
    expect(daemonQueueMoveTarget(queue.items, "a", "down")).toEqual({ beforeItemId: "c" });
    expect(daemonQueueMoveTarget(queue.items, "b", "down")).toEqual({ beforeItemId: null });
    expect(daemonQueueMoveTarget(queue.items, "c", "down")).toBeNull();
  });

  it("enqueues trimmed text with its images, and skips an empty message", async () => {
    const client = createClient();
    const encodeImages = vi.fn(async () => [{ data: "aW1n", mimeType: "image/png" }]);
    const metadata: AttachmentMetadata = {
      id: "img-1",
      mimeType: "image/png",
      storageType: "web-indexeddb",
      storageKey: "img-1",
      createdAt: 0,
    };

    await expect(
      enqueueDaemonComposerMessage({
        client,
        agentId: "agent-1",
        text: "   ",
        attachments: [],
        encodeImages,
      }),
    ).resolves.toBe(false);
    expect(client.enqueueAgentMessage).not.toHaveBeenCalled();

    await expect(
      enqueueDaemonComposerMessage({
        client,
        agentId: "agent-1",
        text: "  Run the tests  ",
        attachments: [{ kind: "image", metadata }],
        encodeImages,
      }),
    ).resolves.toBe(true);
    expect(client.enqueueAgentMessage).toHaveBeenCalledWith("agent-1", {
      text: "Run the tests",
      images: [{ data: "aW1n", mimeType: "image/png" }],
    });
  });

  it("takes a message back with its images as composer attachments", async () => {
    const client = createClient();
    client.cancelQueuedAgentMessage.mockResolvedValueOnce({
      item: { text: "Second", images: [{ data: "aW1n", mimeType: "image/png" }] },
    });
    const persisted: AttachmentMetadata = {
      id: "restored",
      mimeType: "image/png",
      storageType: "web-indexeddb",
      storageKey: "restored",
      createdAt: 0,
    };
    const persistFromDataUrl = vi.fn(async () => persisted);

    await expect(
      takeBackDaemonQueuedMessage({
        client,
        agentId: "agent-1",
        itemId: "b",
        persistFromDataUrl,
      }),
    ).resolves.toEqual({
      text: "Second",
      attachments: [{ kind: "image", metadata: persisted }],
      failedImageCount: 0,
    });
    expect(persistFromDataUrl).toHaveBeenCalledWith({
      dataUrl: "data:image/png;base64,aW1n",
      mimeType: "image/png",
      fileName: null,
    });

    await expect(
      takeBackDaemonQueuedMessage({ client, agentId: "agent-1", itemId: "b", persistFromDataUrl }),
    ).resolves.toBeNull();
  });

  it("keeps the text and the saved images when one image fails to save", async () => {
    const client = createClient();
    client.cancelQueuedAgentMessage.mockResolvedValueOnce({
      item: {
        text: "Two images",
        images: [
          { data: "Zmlyc3Q=", mimeType: "image/png" },
          { data: "c2Vjb25k", mimeType: "image/png" },
        ],
      },
    });
    const saved: AttachmentMetadata = {
      id: "saved",
      mimeType: "image/png",
      storageType: "web-indexeddb",
      storageKey: "saved",
      createdAt: 0,
    };
    const persistFromDataUrl = vi
      .fn()
      .mockResolvedValueOnce(saved)
      .mockRejectedValueOnce(new Error("QuotaExceededError"));

    await expect(
      takeBackDaemonQueuedMessage({ client, agentId: "agent-1", itemId: "b", persistFromDataUrl }),
    ).resolves.toEqual({
      text: "Two images",
      attachments: [{ kind: "image", metadata: saved }],
      failedImageCount: 1,
    });
  });
});
