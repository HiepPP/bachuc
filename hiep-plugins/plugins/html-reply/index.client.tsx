import type { PluginClientContext } from "@getpaseo/plugin/client";
import { HtmlReplyCard } from "./client/card";
import { HTML_REPLY_KIND, HTML_REPLY_VERSION, htmlReplyRowSchema } from "./shared/row";

export default function contribute(client: PluginClientContext) {
  return client.addTimelineRenderer({
    kind: HTML_REPLY_KIND,
    version: HTML_REPLY_VERSION,
    schema: htmlReplyRowSchema,
    Component: HtmlReplyCard,
  });
}
