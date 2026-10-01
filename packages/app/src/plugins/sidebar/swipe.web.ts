import { useEffect, type RefObject } from "react";
import { usePluginSidebarSwipeHandler } from "./index";
import { createWheelGesture } from "./wheel-gesture";

// Pixel, line, and page wheel delta modes.
const WHEEL_DELTA_SCALE: Record<number, number> = { 0: 1, 1: 16, 2: 800 };

interface ScrollableRef {
  getScrollableNode?: () => HTMLElement | null;
}

/** Sends horizontal trackpad swipes over the project list to plugin sidebar filters. */
export function usePluginSidebarSwipe(target: RefObject<unknown>): void {
  const onSwipe = usePluginSidebarSwipeHandler();
  useEffect(() => {
    if (!onSwipe) return undefined;
    const ref = target.current as (ScrollableRef & Partial<HTMLElement>) | null;
    const node = ref?.getScrollableNode?.() ?? (ref instanceof HTMLElement ? ref : null);
    if (!node) return undefined;
    const gesture = createWheelGesture();
    const listener = (event: WheelEvent) => {
      // A held button means a drag is in flight, so the swipe must not switch Spaces under it.
      if (event.ctrlKey || event.buttons !== 0) return;
      if (Math.abs(event.deltaX) > Math.abs(event.deltaY)) event.preventDefault();
      const scale = WHEEL_DELTA_SCALE[event.deltaMode] ?? 1;
      const direction = gesture(event.deltaX * scale, event.deltaY * scale, Date.now());
      if (direction) onSwipe(direction);
    };
    node.addEventListener("wheel", listener, { passive: false });
    return () => node.removeEventListener("wheel", listener);
  }, [onSwipe, target]);
}
