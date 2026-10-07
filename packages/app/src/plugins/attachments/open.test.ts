import { describe, expect, it, vi } from "vitest";
import type { InstalledPlugin } from "../types";
import { createPluginResourceAttachment } from "./model";
import { openPluginResourceAttachment } from "./open";

const item = {
  id: "q1",
  identifier: "quote",
  title: "Quote",
  url: "https://example.com/q1",
  text: "> quoted",
  resourceType: "quote",
};
const chip = createPluginResourceAttachment(
  { pluginId: "cite", sourceId: "quote", sourceTitle: "Quote", sourceIcon: "Quote" },
  item,
);

function plugin(serverId: string, onOpen?: (value: typeof item) => void): InstalledPlugin {
  return {
    id: "cite",
    serverId,
    attachmentSources: [{ id: "quote", title: "Quote", icon: "Quote", onOpen }],
  } as unknown as InstalledPlugin;
}

describe("openPluginResourceAttachment", () => {
  it("calls the onOpen of the chip's source on the composer's host", async () => {
    const onOpen = vi.fn();
    const other = vi.fn();

    expect(
      openPluginResourceAttachment([plugin("other", other), plugin("host", onOpen)], "host", chip),
    ).toBe(true);
    await vi.waitFor(() => expect(onOpen).toHaveBeenCalledWith(item));
    expect(other).not.toHaveBeenCalled();
  });

  it("leaves the press to the URL when the source has no onOpen or is gone", () => {
    expect(openPluginResourceAttachment([plugin("host")], "host", chip)).toBe(false);
    expect(openPluginResourceAttachment([], "host", chip)).toBe(false);
  });
});
