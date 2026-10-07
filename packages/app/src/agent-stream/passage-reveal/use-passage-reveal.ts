import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  registerTimelinePassageRevealer,
  type TimelinePassageRequest,
} from "@/plugins/timeline-reveal";
import { getStreamItemMessageId } from "../presentation";
import type { TimelinePassageRevealProps } from "./types";

/** Starts one reveal and returns how to cancel it. `notLoaded` shows the not-loaded toast. */
export type RunPassageReveal = (
  request: TimelinePassageRequest,
  getProps: () => TimelinePassageRevealProps,
  notLoaded: () => void,
) => (() => void) | void;

/**
 * Registers this stream as the agent's passage revealer. A request waits until the authoritative
 * history is ready, because a stream mounts before its items arrive. It mounts once per stream and
 * keeps the latest props in a ref, so rows and renders gain no effect or subscription.
 */
export function usePassageReveal(props: TimelinePassageRevealProps, run: RunPassageReveal): void {
  const { t } = useTranslation();
  const latest = useRef({ props, run, notLoaded: t("agentStream.passageNotLoaded") });
  latest.current = { props, run, notLoaded: t("agentStream.passageNotLoaded") };
  const flushRef = useRef<() => void>(() => undefined);
  const { serverId, agentId, historyReady } = props;

  useEffect(() => {
    let held: TimelinePassageRequest | null = null;
    let cancel: (() => void) | void;
    const getProps = () => latest.current.props;
    const notLoaded = () => {
      latest.current.props.toast?.show(latest.current.notLoaded);
    };
    const flush = () => {
      const current = latest.current;
      if (!held || !current.props.historyReady) return;
      const request = held;
      held = null;
      cancel?.();
      cancel = undefined;
      const loaded = current.props.items.some(
        (item) => getStreamItemMessageId(item) === request.messageId,
      );
      if (!loaded) {
        notLoaded();
        return;
      }
      cancel = current.run(request, getProps, notLoaded);
    };
    flushRef.current = flush;
    const unregister = registerTimelinePassageRevealer(serverId, agentId, (request) => {
      held = request;
      flush();
    });
    return () => {
      unregister();
      cancel?.();
      flushRef.current = () => undefined;
    };
  }, [agentId, serverId]);

  useEffect(() => {
    if (historyReady) flushRef.current();
  }, [historyReady]);
}
