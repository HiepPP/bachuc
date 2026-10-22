import { MAX_SELECTION_LENGTH } from "../shared/contracts";
import type { TranslateSettings } from "../shared/settings";
import type { Complete } from "./llm";
import { hash, type Store } from "./store";

export function createService(deps: {
  store: Store;
  complete: Complete;
  settings: () => Promise<TranslateSettings>;
}) {
  return {
    async translate({ text: raw }: { text: string }) {
      const text = raw.trim();
      if (!text) throw new Error("Selection is empty");
      if (text.length > MAX_SELECTION_LENGTH)
        throw new Error(`Selection is longer than ${MAX_SELECTION_LENGTH} characters`);
      const { provider, model } = await deps.settings();
      const translation = await deps.store.run(hash(provider, model, text), () =>
        deps.complete({ provider, model, text }),
      );
      return { translation };
    },
  };
}
