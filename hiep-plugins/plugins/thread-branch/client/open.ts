import * as pluginClient from "@getpaseo/plugin/client";
import { Linking } from "react-native";

/** Prefer the host opener (system browser on Electron); older hosts fall back to React Native Linking. */
export async function openUrl(url: string) {
  const opener = (pluginClient as { openExternalUrl?: (url: string) => Promise<void> })
    .openExternalUrl;
  if (typeof opener === "function") {
    await opener(url);
    return;
  }
  await Linking.openURL(url);
}
