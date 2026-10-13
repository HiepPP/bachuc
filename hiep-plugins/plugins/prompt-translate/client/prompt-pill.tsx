import type {
  PluginButtonLabelProps,
  PluginButtonSelection,
  PluginButtonSelectOption,
} from "@getpaseo/plugin/client";
import { modes } from "./modes";
import { usePromptPrefs } from "./prefs";

const modeOptions: readonly PluginButtonSelectOption[] = modes.map(({ value, label }) => ({
  id: value,
  label,
}));

/** The pill names the Caveman mode of its own composer. */
export function PromptPillLabel(props: PluginButtonLabelProps) {
  const { prefs } = usePromptPrefs(props);
  const mode = modes.find((candidate) => candidate.value === prefs?.mode);
  return mode?.label ?? "Default";
}

// The host draws the list for the shared composer pill; the context names the composer's agent.
export function useCavemanSelection(props: PluginButtonLabelProps): PluginButtonSelection {
  const { prefs, save } = usePromptPrefs(props);
  return {
    options: modeOptions,
    value: prefs?.mode ?? null,
    select: (id) => {
      const mode = modes.find((candidate) => candidate.value === id);
      if (mode) save({ mode: mode.value });
    },
  };
}
