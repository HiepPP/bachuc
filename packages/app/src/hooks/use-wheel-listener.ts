import type { View } from "react-native";

export interface WheelEventLike {
  deltaY: number;
  timeStamp: number;
  preventDefault: () => void;
}

export type WheelHandler = (event: WheelEventLike) => void;

/** Called with true when a mouse enters the node and false when it leaves. */
export type HoverHandler = (hovered: boolean) => void;

// Only web has a mouse wheel; use-wheel-listener.web.ts holds the listener.
function ignoreWheelNode(_node: View | null): void {}

export function useWheelListener(
  _handler: WheelHandler | null | undefined,
  _onHover?: HoverHandler | null,
): (node: View | null) => void {
  return ignoreWheelNode;
}
