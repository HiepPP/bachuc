interface ShortcutEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  repeat: boolean;
  isComposing: boolean;
  preventDefault(): void;
  stopImmediatePropagation(): void;
}
interface ShortcutTarget {
  addEventListener(
    type: "keydown",
    listener: (event: ShortcutEvent) => void,
    capture: boolean,
  ): void;
  removeEventListener(
    type: "keydown",
    listener: (event: ShortcutEvent) => void,
    capture: boolean,
  ): void;
}
declare const window: ShortcutTarget;
declare const navigator: { userAgent: string };

export function bindBoardShortcut(target: ShortcutTarget, open: () => void) {
  const listener = (event: ShortcutEvent) => {
    if (
      !event.metaKey ||
      event.ctrlKey ||
      event.altKey ||
      event.shiftKey ||
      event.isComposing ||
      event.key.toLowerCase() !== "d"
    )
      return;
    // Consume the requested chord without forwarding it to focused controls.
    event.preventDefault();
    event.stopImmediatePropagation();
    if (!event.repeat) open();
  };
  target.addEventListener("keydown", listener, true);
  return () => target.removeEventListener("keydown", listener, true);
}

export function installBoardShortcut(open: () => void) {
  if (
    typeof window === "undefined" ||
    typeof navigator === "undefined" ||
    !/Electron\//.test(navigator.userAgent) ||
    !/Macintosh|Mac OS X/.test(navigator.userAgent)
  )
    return () => {};
  return bindBoardShortcut(window, open);
}
