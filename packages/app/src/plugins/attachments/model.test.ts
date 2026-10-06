import { describe, expect, it } from "vitest";
import {
  createPluginResourceAttachment,
  PluginResourceComposerAttachmentSchema,
  pluginResourceAttachmentToAgentAttachment,
  togglePluginResourceAttachment,
  upsertPluginResourceAttachment,
} from "./model";
import { splitComposerAttachmentsForSubmit } from "@/composer/attachments/submit";

const source = {
  pluginId: "linear",
  sourceId: "issues",
  sourceTitle: "Linear issue",
  sourceIcon: "CircleDot",
};

const item = {
  id: "issue-uuid",
  identifier: "ENG-123",
  title: "Plugin attachments",
  subtitle: "In progress · Mohamed",
  url: "https://linear.app/acme/issue/ENG-123/plugin-attachments",
  text: "Linear issue ENG-123: Plugin attachments\nStatus: In progress",
  resourceType: "issue",
};

describe("plugin resource attachments", () => {
  it("adds a chip from code, or replaces the same chip in place", () => {
    const first = createPluginResourceAttachment(source, item);
    const other = createPluginResourceAttachment(source, {
      ...item,
      id: "other",
      identifier: "ENG-9",
    });
    const updated = { ...first, commentable: true, comment: "Look here" };

    expect(upsertPluginResourceAttachment([], first)).toEqual([first]);
    expect(upsertPluginResourceAttachment([first, other], updated)).toEqual([updated, other]);
  });

  it("keeps the user's comment when a plugin adds the same chip again", () => {
    const readded = { ...createPluginResourceAttachment(source, item), commentable: true };
    const commented = { ...readded, comment: "Why this order?" };

    expect(upsertPluginResourceAttachment([commented], readded)).toEqual([commented]);
  });

  it("sends a comment only for a chip that takes one", () => {
    const attachment = { ...createPluginResourceAttachment(source, item), comment: "stale" };
    expect(pluginResourceAttachmentToAgentAttachment(attachment)).toMatchObject({
      text: item.text,
    });
  });

  it("sends a chip comment after the item text, and nothing extra without one", () => {
    const attachment = createPluginResourceAttachment(source, item);
    expect(pluginResourceAttachmentToAgentAttachment(attachment)).toMatchObject({
      text: item.text,
    });
    expect(
      pluginResourceAttachmentToAgentAttachment({
        ...attachment,
        commentable: true,
        comment: "  Why this order?  ",
      }),
    ).toMatchObject({ text: `${item.text}\n\nComment: Why this order?` });
    expect(
      pluginResourceAttachmentToAgentAttachment({
        ...attachment,
        commentable: true,
        comment: "  ",
      }),
    ).toMatchObject({ text: item.text });
  });

  it("keeps the comment fields through the draft schema", () => {
    const attachment = {
      ...createPluginResourceAttachment(source, item),
      commentable: true,
      comment: "x",
    };
    expect(PluginResourceComposerAttachmentSchema.parse(attachment)).toEqual(attachment);
  });

  it("creates a draft-safe composer attachment", () => {
    const attachment = createPluginResourceAttachment(source, item);

    expect(PluginResourceComposerAttachmentSchema.parse(attachment)).toEqual({
      kind: "plugin_resource",
      ...source,
      item,
    });
  });

  it("toggles one resource by plugin, source, and stable item id", () => {
    const attachment = createPluginResourceAttachment(source, item);

    expect(togglePluginResourceAttachment([], attachment)).toEqual([attachment]);
    expect(togglePluginResourceAttachment([attachment], attachment)).toEqual([]);
  });

  it("submits through the backward-compatible text attachment", () => {
    const attachment = createPluginResourceAttachment(source, item);

    const agentAttachment = pluginResourceAttachmentToAgentAttachment(attachment);
    expect(agentAttachment).toEqual({
      type: "text",
      mimeType: "text/plain",
      title: "ENG-123 Plugin attachments",
      text: item.text,
      externalResource: {
        provider: "linear",
        providerLabel: "Linear issue",
        resourceType: "issue",
        id: "issue-uuid",
        identifier: "ENG-123",
        title: "Plugin attachments",
        url: item.url,
      },
    });
    expect(splitComposerAttachmentsForSubmit([attachment])).toEqual({
      images: [],
      attachments: [agentAttachment],
    });
  });
});
