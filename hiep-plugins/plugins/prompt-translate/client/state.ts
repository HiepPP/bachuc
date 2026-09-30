import { translateSettings, type TranslateSettings } from "../shared/settings";

// Adapters read synchronously; the settings screen and the startup read keep this current.
export const current = {
  values: translateSettings.schema.parse({}) as TranslateSettings,
  activeSince: Date.now(),
  onChange: undefined as (() => void) | undefined,
  apply(next: TranslateSettings) {
    // Re-enabling must not translate prompts sent while translation was off.
    if (next.translate && !this.values.translate) this.activeSince = Date.now();
    this.values = next;
    this.onChange?.();
  },
};
