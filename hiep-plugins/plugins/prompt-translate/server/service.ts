import { hasVietnamese } from "../shared/vietnamese";
import { preserveCavemanCommand, stripCavemanMode } from "../shared/caveman";
import type { TranslateSettings } from "../shared/settings";
import type { Complete } from "./llm";
import { hash, type Store } from "./store";

// Old rewrites may omit response-language requests; do not reuse them after changing ENHANCE.
const ENHANCE_CACHE_MODE = "enhance-v2";
export function createService(deps: {
  store: Store;
  complete: Complete;
  settings: () => Promise<TranslateSettings>;
}) {
  return {
    async translate({ text, cacheOnly }: { text: string; cacheOnly: boolean }) {
      if (!hasVietnamese(text)) throw new Error("Text has no Vietnamese to translate");
      const settings = await deps.settings();
      if (!settings.translate) return { translation: null };
      const { provider, translateModel: model } = settings;
      const key = hash("translate", provider, model, text);
      if (cacheOnly) return { translation: (await deps.store.get(key)) ?? null };
      const translation = await deps.store.run(key, () =>
        deps.complete({ mode: "translate", provider, model, text }),
      );
      return { translation };
    },
    async enhance({ text }: { text: string; deferCaveman?: boolean }) {
      const settings = await deps.settings();
      if (!settings.enhanceShortcut) throw new Error("Enhance shortcut is off");
      const { provider, enhanceModel: model } = settings;
      const enhanced = await deps.store.run(hash(ENHANCE_CACHE_MODE, provider, model, text), () =>
        deps.complete({ mode: "enhance", provider, model, text }),
      );
      const prompt = preserveCavemanCommand(enhanced, text);
      await deps.store.pair(prompt, text);
      return { prompt };
    },
    async original({ text }: { text: string }) {
      return {
        original:
          (await deps.store.original(text)) ?? (await deps.store.original(stripCavemanMode(text))),
      };
    },
  };
}
