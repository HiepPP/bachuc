import { AccessibilityInfo, Platform } from "react-native";

let nativeReduced = false;
// Native answers asynchronously; ask at load so the answer is known before the Board mounts.
if (Platform.OS !== "web")
  void AccessibilityInfo?.isReduceMotionEnabled?.().then((value) => {
    nativeReduced = value;
  });

/** True when the user asked the system for reduced motion; every Board animation checks it. */
export function reducedMotion(): boolean {
  if (Platform.OS !== "web") return nativeReduced;
  return (
    (globalThis as { matchMedia?: (query: string) => { matches: boolean } }).matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches === true
  );
}
