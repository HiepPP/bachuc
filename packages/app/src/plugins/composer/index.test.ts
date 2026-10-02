import { PluginComposerDraftState } from "./draft";
import { describe, expect, it, vi } from "vitest";
import { registerComposerHandle, runComposerInterceptors, setPluginComposerText } from "./index";

const target = { serverId: "host-1", workspaceId: "workspace-1", agentId: "agent-1" };

describe("plugin composer interceptors", () => {
  it("chains text through interceptors in order", async () => {
    const seen: string[] = [];
    const outcome = await runComposerInterceptors(
      [
        { id: "first", intercept: ({ text }) => ({ text: `${text} one` }) },
        { id: "skip", intercept: () => undefined },
        {
          id: "second",
          intercept: async ({ text }) => {
            seen.push(text);
            return { text: `${text} two` };
          },
        },
      ],
      { target, text: "start", action: "send" },
    );
    expect(outcome).toEqual({ kind: "send", text: "start one two" });
    expect(seen).toEqual(["start one"]);
  });

  it("stops at a cancel", async () => {
    const later = vi.fn();
    const outcome = await runComposerInterceptors(
      [
        { id: "cancel", intercept: () => ({ cancel: true as const }) },
        { id: "later", intercept: later },
      ],
      { target, text: "start", action: "queue" },
    );
    expect(outcome).toEqual({ kind: "cancel" });
    expect(later).not.toHaveBeenCalled();
  });

  it("propagates an interceptor error", async () => {
    await expect(
      runComposerInterceptors(
        [
          {
            id: "broken",
            intercept: () => {
              throw new Error("broken");
            },
          },
        ],
        { target, text: "start", action: "send" },
      ),
    ).rejects.toThrow("broken");
  });
});

describe("plugin composer text", () => {
  it("replaces the text of the mounted composer for the agent and focuses it", () => {
    const setText = vi.fn();
    const focus = vi.fn();
    const other = vi.fn();
    const remove = registerComposerHandle("host-1", "agent-1", { setText, focus });
    const removeOther = registerComposerHandle("host-1", "agent-2", { setText: other, focus });
    setPluginComposerText("host-1", "agent-1", "Next prompt");
    expect(setText).toHaveBeenCalledWith("Next prompt");
    expect(focus).toHaveBeenCalledTimes(1);
    expect(other).not.toHaveBeenCalled();
    remove();
    removeOther();
  });
});

describe("new workspace plugin preferences", () => {
  it("snapshots synchronous choices for first-send interception and creation, isolated by draft", async () => {
    const draft = new PluginComposerDraftState("host-1:draft-1");
    const otherHost = new PluginComposerDraftState("host-2:draft-1");
    const otherDraft = new PluginComposerDraftState("host-1:draft-2");
    draft.forPlugin("prompt-translate").set({ mode: "ultra", rewrite: false });
    draft.forPlugin("skill-pins").set({ skills: [] });
    const snapshot = draft.snapshot();
    const outcome = await runComposerInterceptors(
      [
        {
          id: "rewrite",
          intercept: ({ target: draftTarget, text }) => ({
            text: (
              draftTarget.draftValues?.["prompt-translate"] as { rewrite: boolean } | undefined
            )?.rewrite
              ? "rewritten"
              : text,
          }),
        },
      ],
      {
        target: { serverId: "host-1", workspaceId: null, agentId: null, draftValues: snapshot },
        text: "Tiếng Việt",
        action: "send",
      },
    );
    expect(outcome).toEqual({ kind: "send", text: "Tiếng Việt" });
    expect(draft.environment()).toEqual({
      "PASEO_PLUGIN_COMPOSER_prompt-translate": '{"mode":"ultra","rewrite":false}',
      "PASEO_PLUGIN_COMPOSER_skill-pins": '{"skills":[]}',
    });
    draft.forPlugin("prompt-translate").set({ mode: "lite", rewrite: true });
    expect(snapshot["prompt-translate"]).toEqual({ mode: "ultra", rewrite: false });
    expect(otherHost.environment()).toEqual({});
    expect(otherDraft.environment()).toEqual({});
  });
});
