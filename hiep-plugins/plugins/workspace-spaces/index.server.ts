import type { PluginServerContext } from "@getpaseo/plugin/server";
import { preferences, catalogRpc } from "./shared/spaces";
import { loadCatalog } from "./server/catalog";
export default function contribute(server: PluginServerContext) {
  server.registerSettings(preferences);
  server.handle(catalogRpc, (_, { paseo }) => loadCatalog(paseo));
  return () => {};
}
