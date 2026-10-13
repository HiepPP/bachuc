/**
 * @vitest-environment jsdom
 */
import { cleanup, renderHook } from "@testing-library/react";
import type { RefObject } from "react";
import type { View } from "react-native";
import { afterEach, describe, expect, it, vi } from "vitest";
import { usePointerLeaveClose } from "./use-pointer-leave-close.web";

afterEach(cleanup);

function nodeAt(rect: { left: number; top: number; right: number; bottom: number }) {
  const node = document.createElement("div");
  node.getBoundingClientRect = () => ({
    ...rect,
    x: 0,
    y: 0,
    width: 0,
    height: 0,
    toJSON: () => ({}),
  });
  return { current: node } as unknown as RefObject<View | null>;
}

function movePointer(clientX: number, clientY: number) {
  document.dispatchEvent(new MouseEvent("pointermove", { clientX, clientY }));
}

// A pill at the bottom left and its list above it, with a 10px gap between them.
const pill = nodeAt({ left: 0, top: 100, right: 50, bottom: 120 });
const list = nodeAt({ left: 0, top: 0, right: 200, bottom: 90 });

describe("usePointerLeaveClose", () => {
  it("keeps the popover open on the pill, the list, and the gap above the pill", () => {
    const onClose = vi.fn();
    renderHook(() => usePointerLeaveClose({ enabled: true, refs: [pill, list], onClose }));

    movePointer(20, 110);
    movePointer(20, 95);
    movePointer(150, 50);

    expect(onClose).not.toHaveBeenCalled();
  });

  it("closes the popover outside them, including on a pill beside the trigger", () => {
    const outside = vi.fn();
    renderHook(() => usePointerLeaveClose({ enabled: true, refs: [pill, list], onClose: outside }));
    movePointer(300, 50);
    expect(outside).toHaveBeenCalledTimes(1);

    cleanup();
    const neighbor = vi.fn();
    renderHook(() =>
      usePointerLeaveClose({ enabled: true, refs: [pill, list], onClose: neighbor }),
    );
    movePointer(80, 110);
    expect(neighbor).toHaveBeenCalledTimes(1);
  });

  it("does nothing while disabled or before every node mounts", () => {
    const onClose = vi.fn();
    const unmounted = { current: null } as RefObject<View | null>;
    renderHook(() => usePointerLeaveClose({ enabled: false, refs: [pill, list], onClose }));
    renderHook(() => usePointerLeaveClose({ enabled: true, refs: [pill, unmounted], onClose }));

    movePointer(300, 50);

    expect(onClose).not.toHaveBeenCalled();
  });
});
