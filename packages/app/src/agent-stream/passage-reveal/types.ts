import type { ReactNode, RefObject } from "react";
import type { ToastApi } from "@/components/toast-host";
import type { StreamItem } from "@/types/stream";
import type { StreamViewportHandle } from "../strategy";

export interface TimelinePassageRevealProps {
  serverId: string;
  agentId: string;
  /** Every loaded item, including the ones outside the rendered history window. */
  items: StreamItem[];
  /** False while the stream still waits for its first authoritative history. */
  historyReady: boolean;
  viewportRef: RefObject<StreamViewportHandle | null>;
  revealLoadedMessage(messageId: string): boolean;
  visibleMessageIds: ReadonlySet<string>;
  toast?: ToastApi | null;
  children?: ReactNode;
}
