import {
  createContext,
  useContext,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { StyleSheet } from "react-native-unistyles";
import type { TimelinePassageRequest } from "@/plugins/timeline-reveal";
import {
  findMessageRows,
  findRenderedMatches,
  findUniqueMessageMatch,
} from "../chat-find/ranges.web";
import type { ScrollToMessageOccurrence } from "../strategy";
import type { TimelinePassageRevealProps } from "./types";
import { usePassageReveal, type RunPassageReveal } from "./use-passage-reveal";

const HIDDEN_ANCHOR: CSSProperties = { display: "none" };
const REVEAL_DEADLINE_MS = 5000;
// `::highlight()` cannot animate, so the highlight clears instead of fading.
const HIGHLIGHT_MS = 2500;
let nextHighlightId = 0;

const RevealTarget = createContext<string | null>(null);

/**
 * The message a passage reveal is working on. The web virtualizer mounts every block row of it,
 * as for chat find, so a duplicate in a far paragraph still counts against a unique match.
 */
export function useRevealTargetMessageId(): string | null {
  return useContext(RevealTarget);
}

interface RevealRun {
  request: TimelinePassageRequest;
  getProps: () => TimelinePassageRevealProps;
  getRoot: () => HTMLElement | null;
  /** True once the virtualizer has been told to mount every row of the message. */
  isPinned: () => boolean;
  highlight: (range: Range) => void;
  notLoaded: () => void;
  done: () => void;
  signal: AbortSignal;
}

function rowOf(range: Range): HTMLElement | null {
  return range.startContainer.parentElement?.closest<HTMLElement>("[data-history-row-id]") ?? null;
}

function hasRenderedRow(root: HTMLElement, messageId: string): boolean {
  return findMessageRows(root, messageId).some((row) => row.getBoundingClientRect().height > 0);
}

/**
 * Brings the message into the history window, waits for its rows to render, and scrolls to it.
 * Text that matches the message once is highlighted and becomes the scroll target.
 */
function revealPassage(run: RevealRun): void {
  const { messageId, text } = run.request;
  const deadline = performance.now() + REVEAL_DEADLINE_MS;
  let scrolled = false;
  let widened = false;
  const scrollToMessage = (occurrence?: ScrollToMessageOccurrence) =>
    run.getProps().viewportRef.current?.scrollToMessage?.(messageId, occurrence);
  const next = () => requestAnimationFrame(poll);
  function poll() {
    if (run.signal.aborted) return;
    if (performance.now() >= deadline) {
      run.notLoaded();
      run.done();
      return;
    }
    const props = run.getProps();
    const root = run.getRoot();
    if (!props.visibleMessageIds.has(messageId)) {
      // A message that the history window cannot show, for example one a transform hides.
      if (!widened && !props.revealLoadedMessage(messageId)) {
        run.notLoaded();
        run.done();
        return;
      }
      widened = true;
      next();
      return;
    }
    // A hidden pane has no height; wait until navigation shows it.
    if (!root?.getBoundingClientRect().height || !run.isPinned()) {
      next();
      return;
    }
    if (!hasRenderedRow(root, messageId)) {
      if (!scrolled) scrollToMessage();
      scrolled = true;
      next();
      return;
    }
    const range = text ? findUniqueMessageMatch(root, messageId, text) : null;
    if (!text || !range) {
      if (!scrolled) scrollToMessage();
      run.done();
      return;
    }
    const row = rowOf(range);
    scrollToMessage({
      signal: run.signal,
      targetTop() {
        const current = row ? findRenderedMatches(row, text)[0] : undefined;
        if (!current) return null;
        run.highlight(current);
        return current.getBoundingClientRect().top;
      },
    });
    run.highlight(range);
  }
  next();
}

/** Reveals a plugin-requested passage in this pane's timeline and briefly highlights it. */
export function TimelinePassageReveal({ children, ...props }: TimelinePassageRevealProps) {
  const anchorRef = useRef<HTMLSpanElement | null>(null);
  const [highlightName] = useState(() => `paseo-passage-reveal-${++nextHighlightId}`);
  const [target, setTarget] = useState<string | null>(null);
  const pinnedRef = useRef<string | null>(null);
  // Runs in the commit that re-renders the virtualizer with the pinned rows.
  useLayoutEffect(() => {
    pinnedRef.current = target;
  }, [target]);
  const run = useMemo<RunPassageReveal>(
    () => (request, getProps, notLoaded) => {
      const controller = new AbortController();
      let timer = 0;
      const clear = () => {
        window.clearTimeout(timer);
        CSS.highlights.delete(highlightName);
      };
      const done = () => {
        clear();
        setTarget(null);
      };
      clear();
      setTarget(request.messageId);
      revealPassage({
        request,
        getProps,
        getRoot: () => anchorRef.current?.parentElement ?? null,
        isPinned: () => pinnedRef.current === request.messageId,
        notLoaded,
        done,
        signal: controller.signal,
        highlight(range) {
          CSS.highlights.set(highlightName, new Highlight(range));
          window.clearTimeout(timer);
          timer = window.setTimeout(done, HIGHLIGHT_MS);
        },
      });
      return () => {
        controller.abort();
        done();
      };
    },
    [highlightName],
  );
  usePassageReveal(props, run);
  return (
    <>
      <span ref={anchorRef} style={HIDDEN_ANCHOR} />
      <style>{`::highlight(${highlightName}) { background-color: ${styles.highlight.backgroundColor}; color: ${styles.highlight.color}; }`}</style>
      <RevealTarget.Provider value={target}>{children}</RevealTarget.Provider>
    </>
  );
}

const styles = StyleSheet.create((theme) => ({
  highlight: { backgroundColor: theme.colors.statusWarning, color: theme.colors.surface0 },
}));
