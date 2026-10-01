import type { TranslateSettings } from "../shared/settings";

export type Mode = TranslateSettings["cavemanMode"];

export const modes: readonly { value: Mode; label: string }[] = [
  { value: "follow-agent", label: "Default" },
  { value: "lite", label: "Lite" },
  { value: "full", label: "Full" },
  { value: "ultra", label: "Ultra" },
  { value: "wenyan-lite", label: "Wenyan Lite" },
  { value: "wenyan-full", label: "Wenyan Full" },
  { value: "wenyan-ultra", label: "Wenyan Ultra" },
];
