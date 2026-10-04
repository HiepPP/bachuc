import { getNextActiveIndex } from "./combobox-keyboard";

export type AutocompleteOptionsPosition = "above-input" | "below-input";

export function orderAutocompleteOptions<T>(
  options: readonly T[],
  position: AutocompleteOptionsPosition = "above-input",
): T[] {
  if (position === "below-input") {
    return [...options];
  }
  return [...options].toReversed();
}

export function getAutocompleteFallbackIndex(
  itemCount: number,
  position: AutocompleteOptionsPosition = "above-input",
): number {
  if (itemCount <= 0) {
    return -1;
  }
  return position === "above-input" ? itemCount - 1 : 0;
}

export function getAutocompleteNextIndex(args: {
  currentIndex: number;
  itemCount: number;
  key: "ArrowDown" | "ArrowUp";
}): number {
  return getNextActiveIndex(args);
}

export function getAutocompleteScrollOffset(args: {
  currentOffset: number;
  viewportHeight: number;
  itemTop: number;
  itemHeight: number;
}): number {
  if (args.viewportHeight <= 0) {
    return args.currentOffset;
  }

  const itemBottom = args.itemTop + args.itemHeight;
  const viewportTop = args.currentOffset;
  const viewportBottom = args.currentOffset + args.viewportHeight;

  if (args.itemTop < viewportTop) {
    return Math.max(0, args.itemTop);
  }

  if (itemBottom > viewportBottom) {
    return Math.max(0, itemBottom - args.viewportHeight);
  }

  return args.currentOffset;
}

interface MeasuredRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface RelativeAnchorRect {
  x: number;
  y: number;
  width: number;
  hostHeight: number;
}

/**
 * Returns null while the anchor or host is not rendered. A composer on a hidden
 * retained screen measures as a zero rect, and placing its popover there paints a
 * zero-width column of text at the window corner.
 */
export function getRelativeAnchorRect(
  anchor: MeasuredRect,
  host: MeasuredRect,
): RelativeAnchorRect | null {
  if (anchor.width <= 0 || anchor.height <= 0 || host.width <= 0 || host.height <= 0) {
    return null;
  }
  return {
    x: anchor.x - host.x,
    y: anchor.y - host.y,
    width: anchor.width,
    hostHeight: host.height,
  };
}
