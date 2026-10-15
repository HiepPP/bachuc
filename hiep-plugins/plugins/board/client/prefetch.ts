/** Hover must rest this long before the card warms its thread; a pass-over costs nothing. */
export const PREFETCH_INTENT_MS = 120;

export interface PrefetchIntent {
  hoverIn(prefetch: () => void): void;
  hoverOut(): void;
  /** A press is a sure open, so it warms at once. */
  pressIn(prefetch: () => void): void;
  cancel(): void;
}

export function createPrefetchIntent(): PrefetchIntent {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const cancel = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
  return {
    hoverIn(prefetch) {
      cancel();
      timer = setTimeout(() => {
        timer = null;
        prefetch();
      }, PREFETCH_INTENT_MS);
    },
    hoverOut: cancel,
    pressIn(prefetch) {
      cancel();
      prefetch();
    },
    cancel,
  };
}
