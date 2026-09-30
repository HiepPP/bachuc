import type { PluginClientContext } from "@getpaseo/plugin/client";
import { Platform } from "react-native";
import { claudeLight } from "./client/theme";
import { injectStyle } from "./client/web";
import { narrowContent } from "./client/width";

type Scope = Parameters<typeof narrowContent>[1] & {
  document: Parameters<typeof injectStyle>[0] & Parameters<typeof narrowContent>[0];
};

export default function contribute(client: PluginClientContext) {
  const removeTheme = client.addTheme(claudeLight);
  if (Platform.OS !== "web") return removeTheme;
  const scope = globalThis as unknown as Scope;
  const removeStyle = injectStyle(scope.document);
  const removeWidth = narrowContent(scope.document, scope);
  return () => {
    removeWidth();
    removeStyle();
    removeTheme();
  };
}
