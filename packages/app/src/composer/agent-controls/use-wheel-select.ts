import type { Ref } from "react";
import type { View } from "react-native";

export interface UseWheelSelectInput {
  enabled: boolean;
  optionIds: readonly string[];
  valueId: string | null | undefined;
  onSelect: (id: string) => void;
}

// Wheel stepping is web-only; use-wheel-select.web.ts holds the listener.
function ignoreWheelNode(_node: View | null): void {}

export function useWheelSelect(_input: UseWheelSelectInput): (node: View | null) => void {
  return ignoreWheelNode;
}

export function composeViewRefs(
  ...refs: readonly (Ref<View> | undefined)[]
): (node: View | null) => void {
  return (node) => {
    for (const ref of refs) {
      if (typeof ref === "function") {
        ref(node);
      } else if (ref) {
        ref.current = node;
      }
    }
  };
}
