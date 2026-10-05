import type { PluginServerContext } from "@getpaseo/plugin/server";
import { withEditToolRule } from "./server/edit-tool-rule";
import { locate } from "./server/locate";
import { locateRpc } from "./shared/contracts";
import { displaySettings } from "./shared/settings";

export default function contribute(server: PluginServerContext) {
  server.registerSettings(displaySettings);
  server.handle(locateRpc, locate);
  const unhook = server.before("agent.create", ({ request }) => withEditToolRule(request));
  return () => unhook();
}
