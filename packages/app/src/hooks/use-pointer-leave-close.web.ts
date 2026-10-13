import { useEffect, useRef, type RefObject } from "react";
import type { View } from "react-native";
import type { PointerLeaveCloseInput } from "./use-pointer-leave-close";

export type { PointerLeaveCloseInput } from "./use-pointer-leave-close";

interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

function contains(box: Box, x: number, y: number): boolean {
  return x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;
}

/**
 * The trigger, each popover, and the strip of the trigger's own column that reaches each popover,
 * so the pointer can cross the gap. A pill beside the trigger stays outside.
 */
function insideBoxes(refs: readonly RefObject<View | null>[]): Box[] | null {
  const boxes: Box[] = [];
  for (const ref of refs) {
    const node: unknown = ref.current;
    // Until every node mounts, nothing counts as outside.
    if (!(node instanceof HTMLElement)) return null;
    boxes.push(node.getBoundingClientRect());
  }
  const [anchor, ...popovers] = boxes;
  if (!anchor) return null;
  const strips = popovers.map((popover) => ({
    left: anchor.left,
    right: anchor.right,
    top: Math.min(anchor.top, popover.top),
    bottom: Math.max(anchor.bottom, popover.bottom),
  }));
  return [...boxes, ...strips];
}

/** Calls `onClose` when the pointer moves outside the trigger, its popover, and the gap between. */
export function usePointerLeaveClose({ enabled, refs, onClose }: PointerLeaveCloseInput): void {
  const liveRef = useRef({ refs, onClose });
  liveRef.current = { refs, onClose };

  useEffect(() => {
    if (!enabled) return undefined;
    const listener = (event: PointerEvent) => {
      const boxes = insideBoxes(liveRef.current.refs);
      if (!boxes) return;
      if (!boxes.some((box) => contains(box, event.clientX, event.clientY))) {
        liveRef.current.onClose();
      }
    };
    document.addEventListener("pointermove", listener);
    return () => document.removeEventListener("pointermove", listener);
  }, [enabled]);
}
