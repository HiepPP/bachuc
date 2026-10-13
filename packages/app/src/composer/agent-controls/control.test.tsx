/**
 * @vitest-environment jsdom
 */
import React from "react";
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AgentControlTrigger } from "./control";
import { resolveComposerControlPresentation } from "./layout";
import { useWheelSelect } from "./use-wheel-select";
import { HOVER_OPEN_DELAY_MS } from "./use-wheel-select";
import { WHEEL_GESTURE_GAP_MS } from "./wheel-select";

beforeEach(() => vi.stubGlobal("React", React));

function TestIcon() {
  return null;
}

function flip(current: boolean): boolean {
  return !current;
}

function waitForHoverOpen() {
  act(() => {
    vi.advanceTimersByTime(HOVER_OPEN_DELAY_MS);
  });
}

// jsdom has no PointerEvent, so set pointerType by hand.
function pointer(node: Element, type: "pointerenter" | "pointerleave", pointerType: string) {
  const event = new MouseEvent(type);
  Object.defineProperty(event, "pointerType", { value: pointerType });
  act(() => {
    node.dispatchEvent(event);
  });
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
    const optionIds = ["low", "medium", "high"];
    // The first event of a gesture steps once at any size.
    const ONE_STEP_DELTA_Y = 60;

    beforeEach(() => {
      vi.useFakeTimers();
      vi.setSystemTime(10_000);
    });
    afterEach(() => {
      cleanup();
      vi.useRealTimers();
    });

    interface WheelPillProps {
      valueId: string;
      open?: boolean;
      disabled?: boolean;
      surface?: "toolbar" | "sheet";
      onSelect: (id: string) => void;
      onOpen: () => void;
    }

    function WheelPill({
      valueId,
      open = false,
      disabled = false,
      surface = "toolbar",
      onSelect,
      onOpen,
    }: WheelPillProps) {
      const wheel = useWheelSelect({ enabled: true, optionIds, valueId, onSelect, open, onOpen });
      return (
        <AgentControlTrigger
          icon={TestIcon}
          surface={surface}
          label="Thinking mode"
          value={valueId}
          open={open}
          disabled={disabled}
          onPress={onOpen}
          accessibilityLabel="Select thinking mode"
          testID="wheel-trigger"
          wheel={wheel}
        />
      );
    }

    function StatefulWheelPill() {
      const [open, setOpen] = React.useState(false);
      const toggle = React.useCallback(() => setOpen(flip), []);
      const wheel = useWheelSelect({
        enabled: true,
        optionIds,
        valueId: "low",
        onSelect: vi.fn(),
        open,
        onOpen: () => setOpen(true),
      });
      return (
        <>
          <AgentControlTrigger
            icon={TestIcon}
            surface="toolbar"
            label="Thinking mode"
            open={open}
            onPress={vi.fn()}
            accessibilityLabel="Select thinking mode"
            testID="wheel-trigger"
            wheel={wheel}
          />
          <button type="button" onClick={toggle}>
            toggle
          </button>
          <span data-testid="opened-by-wheel">{String(wheel.openedByPointer)}</span>
        </>
      );
    }

    function renderWheelPill(props: Omit<WheelPillProps, "onSelect" | "onOpen">) {
      const onSelect = vi.fn();
      const onOpen = vi.fn();
      const view = render(<WheelPill {...props} onSelect={onSelect} onOpen={onOpen} />);
      const rerender = (next: Omit<WheelPillProps, "onSelect" | "onOpen">) =>
        view.rerender(<WheelPill {...next} onSelect={onSelect} onOpen={onOpen} />);
      return { onSelect, onOpen, rerender, node: view.getByTestId("wheel-trigger") };
    }

    it("opens the list and selects the next option on one tick down", () => {
      const { onSelect, onOpen, node } = renderWheelPill({ valueId: "low" });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onOpen).toHaveBeenCalledTimes(1);
      expect(onSelect).toHaveBeenCalledTimes(1);
      expect(onSelect).toHaveBeenCalledWith("medium");
    });

    it("keeps stepping without reopening while the list is open", () => {
      const { onSelect, onOpen, node } = renderWheelPill({ valueId: "low", open: true });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onOpen).not.toHaveBeenCalled();
      expect(onSelect).toHaveBeenCalledWith("medium");
    });

    it("opens the list but keeps the last option on a tick down at the end", () => {
      const { onSelect, onOpen, node } = renderWheelPill({ valueId: "high" });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onOpen).toHaveBeenCalledTimes(1);
      expect(onSelect).not.toHaveBeenCalled();
    });

    it("keeps the first option on a tick up at the start", () => {
      const { onSelect, node } = renderWheelPill({ valueId: "low" });

      fireEvent.wheel(node, { deltaY: -ONE_STEP_DELTA_Y });

      expect(onSelect).not.toHaveBeenCalled();
    });

    it("takes one step for one small notch", () => {
      const { onSelect, node } = renderWheelPill({ valueId: "low" });

      fireEvent.wheel(node, { deltaY: 4 });

      expect(onSelect).toHaveBeenCalledTimes(1);
      expect(onSelect).toHaveBeenCalledWith("medium");
    });

    it("steps from the picked option before the value prop catches up", () => {
      const { onSelect, node } = renderWheelPill({ valueId: "low" });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });
      vi.advanceTimersByTime(WHEEL_GESTURE_GAP_MS);
      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).toHaveBeenNthCalledWith(1, "medium");
      expect(onSelect).toHaveBeenNthCalledWith(2, "high");
    });

    it("steps from the latest value after a rerender", () => {
      const { onSelect, rerender, node } = renderWheelPill({ valueId: "low" });

      rerender({ valueId: "medium" });
      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).toHaveBeenCalledTimes(1);
      expect(onSelect).toHaveBeenCalledWith("high");
    });

    it("ignores the wheel on the sheet surface", () => {
      const { onSelect, node } = renderWheelPill({ valueId: "low", surface: "sheet" });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).not.toHaveBeenCalled();
    });

    it("opens the list after a mouse rests on the pill, not on a touch or a quick pass", () => {
      const { onSelect, onOpen, node } = renderWheelPill({ valueId: "low" });

      pointer(node, "pointerenter", "touch");
      waitForHoverOpen();
      pointer(node, "pointerenter", "mouse");
      pointer(node, "pointerleave", "mouse");
      waitForHoverOpen();
      expect(onOpen).not.toHaveBeenCalled();

      pointer(node, "pointerenter", "mouse");
      waitForHoverOpen();
      expect(onOpen).toHaveBeenCalledTimes(1);
      expect(onSelect).not.toHaveBeenCalled();
    });

    it("marks a list the wheel opened, until the list closes", () => {
      const view = render(<StatefulWheelPill />);
      const openedByWheel = () => view.getByTestId("opened-by-wheel").textContent;

      fireEvent.wheel(view.getByTestId("wheel-trigger"), { deltaY: ONE_STEP_DELTA_Y });
      expect(openedByWheel()).toBe("true");

      fireEvent.click(view.getByText("toggle"));
      expect(openedByWheel()).toBe("false");

      fireEvent.click(view.getByText("toggle"));
      expect(openedByWheel()).toBe("false");
    });

    it("ignores the wheel on a disabled trigger", () => {
      const { onSelect, node } = renderWheelPill({ valueId: "low", disabled: true });

      fireEvent.wheel(node, { deltaY: ONE_STEP_DELTA_Y });

      expect(onSelect).not.toHaveBeenCalled();
    });
  });
});
