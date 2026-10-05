import type { PluginClientContext } from "@getpaseo/plugin/client";
import { Platform } from "react-native";
import { StopOrb } from "./client/stop-orb";

export default function contribute(client: PluginClientContext) {
  // thinking-orbs draws on a DOM canvas, so native keeps Paseo's red stop button.
  if (Platform.OS !== "web") return () => {};
  return client.addComposerStopButton({ id: "solving-orb", Component: StopOrb });
}
