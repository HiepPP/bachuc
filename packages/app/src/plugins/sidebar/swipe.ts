import type { RefObject } from "react";

/** Trackpad swipes are desktop-only; native clients have no wheel events here. */
export function usePluginSidebarSwipe(_target: RefObject<unknown>): void {}
