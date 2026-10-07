import type { PluginServerContext } from "@getpaseo/plugin/server";
import { randomUUID } from "node:crypto";
import { homedir } from "node:os";
import path from "node:path";
import { z } from "zod";
import { createHtmlStore, htmlProblem } from "./server/store";
import {
  cleanTitle,
  getHtmlReplyRpc,
  HTML_REPLY_KIND,
  HTML_REPLY_VERSION,
  MAX_TITLE_LENGTH,
} from "./shared/row";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const store = createHtmlStore(path.join(home, "plugin-data/html-reply"));

  server.handle(getHtmlReplyRpc, async ({ id }) => ({ html: await store.read(id) }));

  server.registerTool({
    name: "publish_html",
    description:
      "Show a self-contained HTML page inline in your thread, for example a chart, table, diagram, or small interactive tool. The page runs in a sandboxed frame that grows to show the whole page, so let the content set its height and do not size it with 100vh; it may load https scripts, styles, fonts, and images and call fetch over https, but it cannot reach the app. Style it with the CSS variables --paseo-background, --paseo-surface, --paseo-foreground, --paseo-muted, --paseo-border, --paseo-accent, --paseo-success, --paseo-warning, --paseo-danger, --paseo-font-ui, and --paseo-font-mono. The HTML may be at most 1 MiB.",
    inputSchema: z.object({
      title: z.string().min(1).max(MAX_TITLE_LENGTH),
      html: z.string(),
    }),
    annotations: {
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: false,
      openWorldHint: false,
    },
    async handler({ title, html }, context) {
      if (!context.callerAgentId) {
        return { text: "publish_html needs a calling agent.", isError: true };
      }
      const problem = htmlProblem(html);
      if (problem) return { text: problem, isError: true };
      const id = randomUUID();
      const row = { id, title: cleanTitle(title) || "HTML page" };
      await store.save(id, html);
      try {
        await context.paseo.agents.ref(context.callerAgentId).timeline.append({
          type: "plugin",
          id,
          kind: HTML_REPLY_KIND,
          version: HTML_REPLY_VERSION,
          data: row,
        });
      } catch (error) {
        await store.remove(id);
        const reason = error instanceof Error ? error.message : String(error);
        return { text: `Could not add the page to your thread: ${reason}`, isError: true };
      }
      return { text: `Published "${row.title}" in your thread.`, structured: { id } };
    },
  });

  return () => undefined;
}
