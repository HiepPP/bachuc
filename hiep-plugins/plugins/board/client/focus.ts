import type { ViewStyle } from "react-native";

type FocusEvent = { target?: unknown; nativeEvent?: { target?: unknown } };

/** Web also focuses on click, so ask the element; native focus only comes from a keyboard. */
export function keyboardFocus(event: unknown): boolean {
  const { target, nativeEvent } = (event ?? {}) as FocusEvent;
  const node = (nativeEvent?.target ?? target) as { matches?: (selector: string) => boolean };
  if (typeof node?.matches !== "function") return true;
  try {
    return node.matches(":focus-visible");
  } catch {
    return true;
  }
}

/** The Board focus ring. Negative offsets keep it inside surfaces that clip their overflow. */
export function focusRing(color: string, offset = 1): ViewStyle {
  return { outlineStyle: "solid", outlineWidth: 2, outlineColor: color, outlineOffset: offset };
}
