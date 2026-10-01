import { describe, expect, it } from "vitest";
import { limitAgentTimelineItemContent } from "./agent-timeline-content.js";
import { markPluginPromptContext, stripPluginPromptContext } from "./plugin-prompt-context.js";

describe("plugin prompt context", () => {
  it("wraps appended text and leaves a replaced prompt alone", () => {
    const marked = markPluginPromptContext("Hi", "Hi\n\nUse the style guide.");
    expect(marked).toBe(
      "Hi\n\n<paseo-plugin-context>\nUse the style guide.\n</paseo-plugin-context>",
    );
    expect(markPluginPromptContext("Xin chào", "Hello")).toBe("Hello");
    expect(markPluginPromptContext("Hi", "Hi")).toBe("Hi");
  });

  it("wraps appended text blocks", () => {
    expect(
      markPluginPromptContext(
        [{ type: "text", text: "Hi" }],
        [
          { type: "text", text: "Hi" },
          { type: "text", text: "Pins" },
        ],
      ),
    ).toEqual([
      { type: "text", text: "Hi" },
      { type: "text", text: "<paseo-plugin-context>\nPins\n</paseo-plugin-context>" },
    ]);
  });

  it("removes wrapped context from replayed user messages", () => {
    const text = markPluginPromptContext("Hi", "Hi\n\nPins") as string;
    expect(stripPluginPromptContext(text)).toBe("Hi");
    expect(limitAgentTimelineItemContent({ type: "user_message", text })).toEqual({
      type: "user_message",
      text: "Hi",
    });
  });
});
