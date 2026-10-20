/**
 * @vitest-environment jsdom
 */
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RetainedPanelActivity } from "@/components/retained-panel";
import { PaneFocusProvider, createPaneFocusContextValue } from "@/panels/pane-context";
import { AssistantSelectionActions } from "./selection-actions.web";

vi.mock("@/plugins/registry", () => ({
  useInstalledPlugins: () => [
    {
      serverId: "host-1",
      assistantSelectionActions: [{ id: "cite", title: "Cite", onSelect: () => {} }],
    },
  ],
}));

vi.mock("react-native-unistyles", () => ({
  StyleSheet: {
    create: (factory: (theme: object) => object) =>
      factory({
        colors: { popover: "white", borderAccent: "gray", foreground: "black" },
        spacing: [0, 4, 8],
        borderRadius: { md: 4, lg: 8 },
        borderWidth: [0, 1],
        shadow: { md: {} },
        fontSize: { sm: 13 },
      }),
  },
}));

beforeEach(() => {
  vi.stubGlobal("React", React);
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  // jsdom has no layout, so a range has no box of its own.
  Range.prototype.getBoundingClientRect = () => container.getBoundingClientRect();
});

let root: Root;
let container: HTMLDivElement;

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  document.body.replaceChildren();
});

function render(input: { active: boolean; workspaceFocused: boolean }) {
  const focus = createPaneFocusContextValue({
    isWorkspaceFocused: input.workspaceFocused,
    isPaneFocused: true,
  });
  act(() =>
    root.render(
      <RetainedPanelActivity active={input.active}>
        <PaneFocusProvider value={focus}>
          <div>
            <AssistantSelectionActions serverId="host-1" agentId="agent-1" workspaceId="w1" />
            <div data-message-id="msg-1">
              <p data-testid="assistant-message">Selected assistant words</p>
            </div>
          </div>
        </PaneFocusProvider>
      </RetainedPanelActivity>,
    ),
  );
}

function selectMessage() {
  const text = container.querySelector("p")?.firstChild;
  if (!text) throw new Error("message text missing");
  act(() => {
    window.getSelection()?.selectAllChildren(text.parentElement as Element);
    document.dispatchEvent(new Event("selectionchange"));
  });
}

function toolbar(): Element | null {
  return document.querySelector('[data-testid="assistant-selection-actions"]');
}

describe("AssistantSelectionActions", () => {
  it("shows the toolbar over a selection in a visible pane", () => {
    render({ active: true, workspaceFocused: true });
    selectMessage();
    expect(toolbar()?.textContent).toBe("Cite");
  });

  it("hides the toolbar when another route, such as Board, covers the workspace", () => {
    render({ active: true, workspaceFocused: true });
    selectMessage();
    render({ active: true, workspaceFocused: false });
    expect(toolbar()).toBeNull();
    render({ active: true, workspaceFocused: true });
    expect(toolbar()?.textContent).toBe("Cite");
  });

  it("hides the toolbar when its tab hides", () => {
    render({ active: true, workspaceFocused: true });
    selectMessage();
    render({ active: false, workspaceFocused: true });
    expect(toolbar()).toBeNull();
  });
});
