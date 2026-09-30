import type { PluginClientContext } from "@getpaseo/plugin/client";

export default function contribute(client: PluginClientContext) {
  return client.addTheme({
    id: "light-plus",
    name: "VS Code Light+ (Warm)",
    appearance: "light",
    colors: {
      background: "#F5F3EE",
      foreground: "#4C4843",
      raised: "#EEEBE3",
      control: "#EEEBE3",
      border: "#DEDACF",
      accent: "#B07B56",
      mutedForeground: "#8C877C",
      ring: "#CBB79E",
    },
  });
}
