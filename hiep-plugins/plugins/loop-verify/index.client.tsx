import type { PluginClientContext } from "@getpaseo/plugin/client";
import { LoopStatusCard } from "./client/status-card";
import { STATUS_KIND, STATUS_VERSION, loopStatusSchema } from "./shared/status";

export default function contribute(client: PluginClientContext) {
  return client.addTimelineRenderer({
    kind: STATUS_KIND,
    version: STATUS_VERSION,
    schema: loopStatusSchema,
    Component: LoopStatusCard,
  });
}
