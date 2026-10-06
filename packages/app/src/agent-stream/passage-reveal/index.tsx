import type { TimelinePassageRevealProps } from "./types";
import { usePassageReveal, type RunPassageReveal } from "./use-passage-reveal";

// The native stream has no `scrollToMessage` and no text highlight, so a reveal opens the agent
// and stops there (Q-010).
const openOnly: RunPassageReveal = () => undefined;

export function TimelinePassageReveal({ children, ...props }: TimelinePassageRevealProps) {
  usePassageReveal(props, openOnly);
  return children;
}

/** Native never pins a reveal target in a virtualizer. */
export function useRevealTargetMessageId(): string | null {
  return null;
}
