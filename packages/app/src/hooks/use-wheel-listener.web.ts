import { useCallback, useEffect, useRef, useState } from "react";
import type { View } from "react-native";
import type { HoverHandler, WheelHandler } from "./use-wheel-listener";

export type { HoverHandler, WheelEventLike, WheelHandler } from "./use-wheel-listener";

/**
 * Attaches a non-passive `wheel` listener to the node while `handler` is set, so the handler can
 * call `preventDefault()` and keep the page behind from scrolling. While `onHover` is set, it also
 * reports a mouse entering and leaving the node; touch pointers are ignored.
 */
export function useWheelListener(
  handler: WheelHandler | null | undefined,
  onHover?: HoverHandler | null,
): (node: View | null) => void {
  const [element, setElement] = useState<HTMLElement | null>(null);
  const handlerRef = useRef(handler);
  handlerRef.current = handler;
  const hoverRef = useRef(onHover);
  hoverRef.current = onHover;
  const active = handler != null;
  const hoverActive = onHover != null;

  const setNode = useCallback((node: View | null) => {
    const raw: unknown = node;
    setElement(raw instanceof HTMLElement ? raw : null);
  }, []);

  useEffect(() => {
    if (!active || !element) return undefined;
    const listener = (event: WheelEvent) => handlerRef.current?.(event);
    element.addEventListener("wheel", listener, { passive: false });
    return () => element.removeEventListener("wheel", listener);
  }, [active, element]);

  useEffect(() => {
    if (!hoverActive || !element) return undefined;
    const enter = (event: PointerEvent) => {
      if (event.pointerType === "mouse") hoverRef.current?.(true);
    };
    const leave = () => hoverRef.current?.(false);
    element.addEventListener("pointerenter", enter);
    element.addEventListener("pointerleave", leave);
    return () => {
      element.removeEventListener("pointerenter", enter);
      element.removeEventListener("pointerleave", leave);
      hoverRef.current?.(false);
    };
  }, [hoverActive, element]);

  return setNode;
}
