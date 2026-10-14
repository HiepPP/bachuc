import path from "node:path";

import { resolvePaseoHome } from "../../../paseo-home.js";
import type {
  ProviderProfileModel,
  ProviderRuntimeSettings,
} from "../../provider-launch-config.js";

export const GLM_BASE_URL = "https://api.z.ai/api/anthropic";
// Z.AI answers slower than Anthropic; this matches the Z.AI Claude Code setup guide.
export const GLM_API_TIMEOUT_MS = "3000000";
export const GLM_DEFAULT_MODEL_ID = "glm-5.3-flash";
export const GLM_MISSING_KEY_REASON =
  "No Z.AI API key. Set ZAI_API_KEY (or GLM_API_KEY) in the daemon environment.";

export const GLM_MODELS: ProviderProfileModel[] = [
  { id: GLM_DEFAULT_MODEL_ID, label: "GLM-5.3 Flash", isDefault: true },
  { id: "glm-5.3", label: "GLM-5.3" },
  { id: "glm-5-turbo", label: "GLM-5 Turbo" },
  { id: "glm-4.7", label: "GLM-4.7" },
];

// WebSearch is an Anthropic server-side tool that Z.AI does not support.
export const GLM_DISALLOWED_TOOLS = ["WebSearch"];

export interface ResolveGlmApiKeyInput {
  providerEnv?: Record<string, string>;
  env?: Record<string, string | undefined>;
}

export interface ResolveGlmRuntimeInput {
  providerEnv?: Record<string, string>;
  env?: NodeJS.ProcessEnv;
}

export interface GlmRuntime {
  apiKey: string | null;
  runtimeSettings: ProviderRuntimeSettings;
}

function nonEmpty(value: string | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

export function resolveGlmApiKey(input: ResolveGlmApiKeyInput = {}): string | null {
  const env = input.env ?? process.env;
  return (
    nonEmpty(input.providerEnv?.ANTHROPIC_AUTH_TOKEN) ??
    nonEmpty(env.ZAI_API_KEY) ??
    nonEmpty(env.GLM_API_KEY)
  );
}

// GLM sessions keep their own Claude config so they never share ~/.claude hooks, plugins, or history.
export function resolveGlmConfigDir(env: NodeJS.ProcessEnv = process.env): string {
  return path.join(resolvePaseoHome(env), "providers", "glm", "claude-config");
}

export function resolveGlmRuntime(input: ResolveGlmRuntimeInput = {}): GlmRuntime {
  const env = input.env ?? process.env;
  const apiKey = resolveGlmApiKey(input);
  return {
    apiKey,
    runtimeSettings: {
      env: {
        CLAUDE_CONFIG_DIR: resolveGlmConfigDir(env),
        ANTHROPIC_BASE_URL: GLM_BASE_URL,
        API_TIMEOUT_MS: GLM_API_TIMEOUT_MS,
        // Claude Code maps subagent and background model aliases through these, so they stay on GLM.
        ANTHROPIC_DEFAULT_OPUS_MODEL: GLM_DEFAULT_MODEL_ID,
        ANTHROPIC_DEFAULT_SONNET_MODEL: GLM_DEFAULT_MODEL_ID,
        ANTHROPIC_DEFAULT_HAIKU_MODEL: GLM_DEFAULT_MODEL_ID,
        ANTHROPIC_SMALL_FAST_MODEL: GLM_DEFAULT_MODEL_ID,
        ...(apiKey ? { ANTHROPIC_AUTH_TOKEN: apiKey } : {}),
      },
      disallowedTools: [...GLM_DISALLOWED_TOOLS],
    },
  };
}
