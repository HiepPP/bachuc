import { describe, expect, it, vi } from "vitest";
import type {
  ProviderSelectionModelRow,
  ProviderSelectorProvider,
} from "@/provider-selection/provider-selection";
import { resolveWheelOptionIndex } from "@/composer/agent-controls/wheel-select";
import {
  isModelWheelEnabled,
  resolveCurrentProviderModelKey,
  resolveCurrentProviderModelKeys,
  splitModelRowKey,
} from "./combined-model-selector";

// The helpers are pure. Stub the selector's UI imports, whose native dependencies
// do not load in the node test environment.
vi.mock("@/components/model-browser", () => ({
  ModelBrowser: () => null,
  ModelProviderGlyph: () => null,
  useModelBrowser: () => ({}),
}));
vi.mock("@/components/ui/combobox", () => ({ Combobox: () => null }));
vi.mock("@/components/ui/combobox-trigger", () => ({ ComboboxTrigger: () => null }));
vi.mock("@/components/ui/loading-spinner", () => ({ LoadingSpinner: () => null }));

function row(provider: string, modelId: string, isDefault = false): ProviderSelectionModelRow {
  return {
    favoriteKey: `${provider}:${modelId}`,
    provider,
    providerLabel: provider,
    modelId,
    modelLabel: modelId,
    isDefault,
  };
}

function modelsProvider(id: string, rows: ProviderSelectionModelRow[]): ProviderSelectorProvider {
  return { id, label: id, modelSelection: { kind: "models", rows } };
}

const providers: ProviderSelectorProvider[] = [
  modelsProvider("claude", [row("claude", "opus"), row("claude", "sonnet", true)]),
  modelsProvider("codex", [row("codex", "gpt-5"), row("codex", "gpt-5-mini"), row("codex", "o3")]),
  { id: "pi", label: "pi", modelSelection: { kind: "loading" } },
  { id: "copilot", label: "copilot", modelSelection: { kind: "error", message: "boom" } },
];

function step(keys: string[], currentKey: string, steps: number): string | undefined {
  const index = resolveWheelOptionIndex({
    currentIndex: keys.indexOf(currentKey),
    steps,
    count: keys.length,
  });
  return keys[index];
}

describe("resolveCurrentProviderModelKeys", () => {
  it("returns the current provider's rows in row order", () => {
    expect(resolveCurrentProviderModelKeys(providers, "codex")).toEqual([
      "codex:gpt-5",
      "codex:gpt-5-mini",
      "codex:o3",
    ]);
  });

  it("returns only keys that start with the selected provider id", () => {
    for (const id of ["claude", "codex"]) {
      const keys = resolveCurrentProviderModelKeys(providers, id);
      expect(keys.length).toBeGreaterThan(0);
      for (const key of keys) {
        expect(key.startsWith(`${id}:`)).toBe(true);
      }
    }
  });

  it("returns an empty list for a loading, error, or unknown provider", () => {
    expect(resolveCurrentProviderModelKeys(providers, "pi")).toEqual([]);
    expect(resolveCurrentProviderModelKeys(providers, "copilot")).toEqual([]);
    expect(resolveCurrentProviderModelKeys(providers, "missing")).toEqual([]);
    expect(resolveCurrentProviderModelKeys(providers, "")).toEqual([]);
  });

  it("clamps at the provider's first and last model without crossing providers", () => {
    const keys = resolveCurrentProviderModelKeys(providers, "codex");
    expect(step(keys, "codex:gpt-5", 1)).toBe("codex:gpt-5-mini");
    expect(step(keys, "codex:o3", 1)).toBe("codex:o3");
    expect(step(keys, "codex:o3", 5)).toBe("codex:o3");
    expect(step(keys, "codex:gpt-5", -1)).toBe("codex:gpt-5");
    expect(step(keys, "codex:gpt-5-mini", -3)).toBe("codex:gpt-5");
  });
});

describe("resolveCurrentProviderModelKey", () => {
  it("returns the key of the selected model", () => {
    expect(
      resolveCurrentProviderModelKey({ providers, selectedProvider: "codex", selectedModel: "o3" }),
    ).toBe("codex:o3");
  });

  it("falls back to the default row, then the first row, like the label", () => {
    expect(
      resolveCurrentProviderModelKey({ providers, selectedProvider: "claude", selectedModel: "" }),
    ).toBe("claude:sonnet");
    expect(
      resolveCurrentProviderModelKey({ providers, selectedProvider: "codex", selectedModel: "" }),
    ).toBe("codex:gpt-5");
  });

  it("returns null for an unknown model id or a provider without rows", () => {
    expect(
      resolveCurrentProviderModelKey({
        providers,
        selectedProvider: "codex",
        selectedModel: "custom",
      }),
    ).toBeNull();
    expect(
      resolveCurrentProviderModelKey({ providers, selectedProvider: "pi", selectedModel: "" }),
    ).toBeNull();
  });
});

describe("isModelWheelEnabled", () => {
  const ready = {
    isComposerToolbar: true,
    isOpen: false,
    disabled: false,
    isLoading: false,
    count: 3,
    valueId: "codex:o3",
  };

  it("is on for a closed, ready composer pill on a listed model", () => {
    expect(isModelWheelEnabled(ready)).toBe(true);
  });

  it("is off outside the composer toolbar", () => {
    expect(isModelWheelEnabled({ ...ready, isComposerToolbar: false })).toBe(false);
  });

  it("is off while the popover is open, disabled, or loading", () => {
    expect(isModelWheelEnabled({ ...ready, isOpen: true })).toBe(false);
    expect(isModelWheelEnabled({ ...ready, disabled: true })).toBe(false);
    expect(isModelWheelEnabled({ ...ready, isLoading: true })).toBe(false);
  });

  it("is off for fewer than two models", () => {
    expect(isModelWheelEnabled({ ...ready, count: 1 })).toBe(false);
    expect(isModelWheelEnabled({ ...ready, count: 0 })).toBe(false);
  });

  it("is off for an off-list model id", () => {
    expect(isModelWheelEnabled({ ...ready, valueId: null })).toBe(false);
  });
});

describe("splitModelRowKey", () => {
  it("splits at the first colon so model ids keep their colons", () => {
    expect(splitModelRowKey("codex:o3")).toEqual({ provider: "codex", modelId: "o3" });
    expect(splitModelRowKey("ollama:llama3:8b")).toEqual({
      provider: "ollama",
      modelId: "llama3:8b",
    });
    expect(splitModelRowKey("claude:")).toEqual({ provider: "claude", modelId: "" });
    expect(splitModelRowKey("nocolon")).toBeNull();
  });
});
