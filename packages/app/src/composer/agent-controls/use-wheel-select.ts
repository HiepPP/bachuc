import { useCallback, useEffect, useRef, useState } from "react";
import type { Ref } from "react";
import type { View } from "react-native";
import type { HoverHandler, WheelEventLike, WheelHandler } from "@/hooks/use-wheel-listener";
import {
  INITIAL_WHEEL_STEP_STATE,
  resolveWheelOptionIndex,
  resolveWheelSteps,
} from "./wheel-select";

// A select reaches the label only after a server round trip. Until the value prop moves, the
// next step starts from the option this hook picked, so fast ticks do not repeat one option.
const PENDING_SELECT_MS = 1500;

export interface UseWheelSelectInput {
  enabled: boolean;
  optionIds: readonly string[];
  valueId: string | null | undefined;
  onSelect: (id: string) => void;
  /** Opens the dropdown on a step while it is closed, so the list shows where the value is. */
  open?: boolean;
  onOpen?: () => void;
}

interface PendingSelect {
  id: string;
  fromValueId: string | null | undefined;
  at: number;
}

function resolveCurrentId(
  valueId: string | null | undefined,
  pending: PendingSelect | null,
  now: number,
): string | null | undefined {
  if (pending && pending.fromValueId === valueId && now - pending.at < PENDING_SELECT_MS) {
    return pending.id;
  }
  return valueId;
}

export interface WheelSelect {
  /** Null while the wheel does nothing. Attach it to the trigger and to the open list's backdrop. */
  handler: WheelHandler | null;
  /** Null while hover does nothing. Attach it to the trigger with `useWheelListener`. */
  onHover: HoverHandler | null;
  /** True while a hover or a wheel step opened the list. Pass it to `closeOnPointerLeave`. */
  openedByPointer: boolean;
}

/** How long the mouse rests on a pill before its list opens. */
export const HOVER_OPEN_DELAY_MS = 150;

/** Opens the list from a hover or a wheel step, and remembers that until the list closes. */
export function usePointerOpen(
  open: boolean | undefined,
  onOpen: (() => void) | undefined,
): { openedByPointer: boolean; openByPointer: () => void; onHover: HoverHandler } {
  const [marked, setMarked] = useState(false);
  useEffect(() => {
    if (!open) setMarked(false);
  }, [open]);
  const liveRef = useRef({ open, onOpen });
  liveRef.current = { open, onOpen };
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const openByPointer = useCallback(() => {
    const live = liveRef.current;
    if (live.open || !live.onOpen) return;
    setMarked(true);
    live.onOpen();
  }, []);
  const onHover = useCallback(
    (hovered: boolean) => {
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = hovered ? setTimeout(openByPointer, HOVER_OPEN_DELAY_MS) : null;
    },
    [openByPointer],
  );
  useEffect(
    () => () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    },
    [],
  );
  return { openedByPointer: open === true && marked, openByPointer, onHover };
}

/** Steps the value on the wheel, and opens the list on a hover or the first step. */
export function useWheelSelect(input: UseWheelSelectInput): WheelSelect {
  const liveRef = useRef(input);
  liveRef.current = input;
  const stepStateRef = useRef(INITIAL_WHEEL_STEP_STATE);
  const pendingRef = useRef<PendingSelect | null>(null);
  const active = input.enabled && input.optionIds.length >= 2;
  const { openedByPointer, openByPointer, onHover } = usePointerOpen(input.open, input.onOpen);

  const handleWheel = useCallback(
    (event: WheelEventLike) => {
      event.preventDefault();
      const result = resolveWheelSteps({
        state: stepStateRef.current,
        deltaY: event.deltaY,
        now: event.timeStamp,
      });
      stepStateRef.current = result.state;
      if (result.steps === 0) return;
      const live = liveRef.current;
      const currentId = resolveCurrentId(live.valueId, pendingRef.current, event.timeStamp);
      const nextIndex = resolveWheelOptionIndex({
        currentIndex: live.optionIds.findIndex((id) => id === currentId),
        steps: result.steps,
        count: live.optionIds.length,
      });
      const nextId = live.optionIds[nextIndex];
      openByPointer();
      if (nextId === undefined || nextId === currentId) return;
      pendingRef.current = { id: nextId, fromValueId: live.valueId, at: event.timeStamp };
      live.onSelect(nextId);
    },
    [openByPointer],
  );

  return {
    handler: active ? handleWheel : null,
    onHover: active ? onHover : null,
    openedByPointer,
  };
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
