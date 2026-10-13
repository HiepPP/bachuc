import type { RefObject } from "react";
import type { View } from "react-native";

export interface PointerLeaveCloseInput {
  enabled: boolean;
  /** The trigger first, then its popover. The gap in the trigger's column counts as inside. */
  refs: readonly RefObject<View | null>[];
  onClose: () => void;
}

// Only web has a hovering pointer; use-pointer-leave-close.web.ts holds the listener.
export function usePointerLeaveClose(_input: PointerLeaveCloseInput): void {}
