/**
 * @vitest-environment jsdom
 */
import React from "react";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AgentControlTrigger } from "./control";
import { resolveComposerControlPresentation } from "./layout";

beforeEach(() => vi.stubGlobal("React", React));

function TestIcon() {
  return null;
}

describe("AgentControlTrigger", () => {
  it.each(["full", "condensed", "tight"] as const)(
    "shows the selected effort in a %s toolbar",
    (density) => {
      const presentation = resolveComposerControlPresentation(density);
      const view = render(
        <AgentControlTrigger
          icon={TestIcon}
          surface="toolbar"
          label="Thinking mode"
          value="Max"
          showToolbarLabel={presentation.showThinkingLabel}
          showCaret={presentation.showCarets}
          onPress={vi.fn()}
          accessibilityLabel="Select thinking mode: Max"
        />,
      );

      expect(view.getByRole("button", { name: "Select thinking mode: Max" }).textContent).toBe(
        "Max",
      );
      view.unmount();
    },
  );

  it("forwards interaction handlers to the rendered trigger", () => {
    const onPointerEnter = vi.fn();
    const onFocus = vi.fn();
    const onPress = vi.fn();
    const view = render(
      <AgentControlTrigger
        icon={TestIcon}
        surface="toolbar"
        label="Mode"
        onPress={onPress}
        onPointerEnter={onPointerEnter}
        onFocus={onFocus}
        accessibilityLabel="Select mode"
      />,
    );
    const trigger = view.getByRole("button", { name: "Select mode" });

    fireEvent.pointerEnter(trigger);
    fireEvent.focus(trigger);

    expect(onPointerEnter).toHaveBeenCalledTimes(1);
    expect(onFocus).toHaveBeenCalledTimes(1);
  });

  describe("wheel select", () => {
    const optionIds = ["low", "medium", "high"] as const;
    // One step needs 48 accumulated pixels, so 60 gives exactly one step.
    const ONE_STEP_DELTA_Y = 60;

    afterEach(cleanup);

    function createWheelSelect(valueId: string, onSelect: (id: string) => void) {
      return { optionIds, valueId, onSelect };
    }

    interface WheelTriggerOptions {
      valueId: string;
      open?: boolean;
      disabled?: boolean;
      surface?: "toolbar" | "sheet";
    }

    function wheelTriggerElement(
      { valueId, open = false, disabled = false, surface = "toolbar" }: WheelTriggerOptions,
      onSelect: (id: string) => void,
    ) {
      const wheelSelect = createWheelSelect(valueId, onSelect);
      return (
        <AgentControlTrigger
          icon={TestIcon}
          surface={surface}
          label="Thinking mode"
          value={valueId}
          open={open}
          disabled={disabled}
          onPress={vi.fn()}
          accessibilityLabel="Select thinking mode"
          testID="wheel-trigger"
          wheelSelect={wheelSelect}
        />
      );
    }

    function renderWheelTrigger(options: WheelTriggerOptions) {
      const onSelect = vi.fn();
      const view = render(wheelTriggerElement(options, onSelect));
      const rerender = (next: WheelTriggerOptions) =>
        view.rerender(wheelTriggerElement(next, onSelect));
      return { onSelect, rerender, node: view.getByTestId("wheel-trigger") };
    }

    it("steps to the next option on one wheel tick down", () => {
      const { onSelect, node } = renderWheelTrigger({ valueId: "low" });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).toHaveBeenCalledTimes(1);
      expect(onSelect).toHaveBeenCalledWith("medium");
    });

    it("keeps the last option on a wheel tick down at the end", () => {
      const { onSelect, node } = renderWheelTrigger({ valueId: "high" });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).not.toHaveBeenCalled();
    });

    it("ignores the wheel while the list is open", () => {
      const { onSelect, node } = renderWheelTrigger({ valueId: "low", open: true });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).not.toHaveBeenCalled();
    });

    it("ignores the wheel on the sheet surface", () => {
      const { onSelect, node } = renderWheelTrigger({ valueId: "low", surface: "sheet" });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).not.toHaveBeenCalled();
    });

    it("ignores the wheel on a disabled trigger", () => {
      const { onSelect, node } = renderWheelTrigger({ valueId: "low", disabled: true });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).not.toHaveBeenCalled();
    });

    it("keeps the first option on a wheel tick up at the start", () => {
      const { onSelect, node } = renderWheelTrigger({ valueId: "low" });

      fireEvent.wheel(node, { deltaY: -ONE_STEP_DELTA_Y });

      expect(onSelect).not.toHaveBeenCalled();
    });

    it("steps from the latest value after a rerender", () => {
      const { onSelect, rerender, node } = renderWheelTrigger({ valueId: "low" });

      rerender({ valueId: "medium" });
      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).toHaveBeenCalledTimes(1);
      expect(onSelect).toHaveBeenCalledWith("high");
    });
  });
});
