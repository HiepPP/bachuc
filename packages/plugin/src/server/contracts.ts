import type { PaseoApi } from "@getpaseo/client";
import type { ZodType, input as ZodInput, output as ZodOutput } from "zod";
import type { PluginRpcContract } from "../rpc.js";
import type { PluginCleanup } from "../contracts.js";
import type { ProviderRegistration } from "./provider.js";
import type { PluginLifecycleRegistration } from "./lifecycle.js";

export interface PluginHandlerContext {
  paseo: PaseoApi;
}

export type PluginSettingsState<Schema extends ZodType> =
  | {
      status: "ready";
      revision: string;
      values: ZodOutput<Schema>;
    }
  | {
      status: "invalid";
      revision: string;
      error: string;
    };

export interface PluginSettings<Schema extends ZodType> {
  read(): Promise<PluginSettingsState<Schema>>;
  subscribe(listener: (state: PluginSettingsState<Schema>) => void | Promise<void>): PluginCleanup;
}

export interface PluginToolContext {
  paseo: PaseoApi;
  /** The agent that called the tool, or null when the caller is not an agent. */
  callerAgentId: string | null;
}

export interface PluginToolResult {
  text: string;
  structured?: Record<string, unknown>;
  isError?: boolean;
}

export interface PluginToolDefinition<InputSchema extends ZodType> {
  /** Served by the daemon `paseo` MCP server under this exact name. */
  name: string;
  description: string;
  /** Must describe a JSON object. */
  inputSchema: InputSchema;
  annotations?: {
    readOnlyHint?: boolean;
    destructiveHint?: boolean;
    idempotentHint?: boolean;
    openWorldHint?: boolean;
  };
  handler(
    input: ZodOutput<InputSchema>,
    context: PluginToolContext,
  ): PluginToolResult | Promise<PluginToolResult>;
}

export interface PluginServerContext extends PluginLifecycleRegistration {
  registerSettings<Schema extends ZodType>(
    definition: import("../settings.js").SettingsDefinition<Schema>,
  ): PluginSettings<Schema>;
  handle<InputSchema extends ZodType, OutputSchema extends ZodType>(
    contract: PluginRpcContract<InputSchema, OutputSchema>,
    handler: (
      input: ZodOutput<InputSchema>,
      context: PluginHandlerContext,
    ) => ZodInput<OutputSchema> | Promise<ZodInput<OutputSchema>>,
  ): void;
  registerProvider(provider: ProviderRegistration): void;
  registerTool<InputSchema extends ZodType>(definition: PluginToolDefinition<InputSchema>): void;
}

export type PluginServerContribution = (server: PluginServerContext) => PluginCleanup;
