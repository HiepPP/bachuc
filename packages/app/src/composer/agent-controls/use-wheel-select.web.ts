import { useCallback, useEffect, useRef, useState } from "react";
import type { Ref } from "react";
import type { View } from "react-native";
import {
  INITIAL_WHEEL_STEP_STATE,
  WHEEL_IDLE_RESET_MS,
  WHEEL_STEP_THRESHOLD_PX,
  resolveWheelOptionIndex,
  resolveWheelSteps,
} from "./wheel-select";

export interface UseWheelSelectInput {
  enabled: boolean;
  optionIds: readonly string[];
  valueId: string | null | undefined;
  onSelect: (id: string) => void;
}

export function useWheelSelect({
  enabled,
  optionIds,
  valueId,
  onSelect,
}: UseWheelSelectInput): (node: View | null) => void {
  const [element, setElement] = useState<HTMLElement | null>(null);
  const liveRef = useRef({ optionIds, valueId, onSelect });
  liveRef.current = { optionIds, valueId, onSelect };
  const active = enabled && optionIds.length >= 2;

  const setNode = useCallback((node: View | null) => {
    const raw: unknown = node;
    setElement(raw instanceof HTMLElement ? raw : null);
  }, []);

  useEffect(() => {
    if (!active || !element) {
      return undefined;
    }
    let stepState = INITIAL_WHEEL_STEP_STATE;
    const handleWheel = (event: WheelEvent) => {
      event.preventDefault();
      const result = resolveWheelSteps({
        state: stepState,
        deltaY: event.deltaY,
        now: event.timeStamp,
        threshold: WHEEL_STEP_THRESHOLD_PX,
        idleMs: WHEEL_IDLE_RESET_MS,
      });
      stepState = result.state;
      if (result.steps === 0) {
        return;
      }
      const live = liveRef.current;
      const nextIndex = resolveWheelOptionIndex({
        currentIndex: live.optionIds.findIndex((id) => id === live.valueId),
        steps: result.steps,
        count: live.optionIds.length,
      });
      const nextId = live.optionIds[nextIndex];
      if (nextId !== undefined && nextId !== live.valueId) {
        live.onSelect(nextId);
      }
    };
    element.addEventListener("wheel", handleWheel, { passive: false });
    return () => {
      element.removeEventListener("wheel", handleWheel);
    };
  }, [active, element]);

  return setNode;
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
