import type {
  PluginComposerInterceptInput,
  PluginComposerInterceptorContribution,
} from "@getpaseo/plugin/client";
import { buildDraftStoreKey } from "@/stores/draft-keys";
import { useDraftStore } from "@/stores/draft-store";

export interface ComposerHandle {
  setText(text: string): void;
  focus(): void;
}

const handles = new Map<string, Set<ComposerHandle>>();

function handleKey(serverId: string, agentId: string): string {
  return `${serverId}\u0000${agentId}`;
}

/** A mounted composer registers itself so plugins can replace its text. */
export function registerComposerHandle(
  serverId: string,
  agentId: string,
  handle: ComposerHandle,
): () => void {
  const key = handleKey(serverId, agentId);
  let set = handles.get(key);
  if (!set) {
    set = new Set();
    handles.set(key, set);
  }
  set.add(handle);
  return () => {
    set.delete(handle);
    if (set.size === 0) handles.delete(key);
  };
}

export function setPluginComposerText(serverId: string, agentId: string, text: string): void {
  const mounted = handles.get(handleKey(serverId, agentId));
  if (mounted && mounted.size > 0) {
    for (const handle of mounted) {
      handle.setText(text);
      handle.focus();
    }
    return;
  }
  // No mounted composer: the stored draft shows the text when the composer next mounts.
  useDraftStore
    .getState()
    .editDraftText({ draftKey: buildDraftStoreKey({ serverId, agentId }), text });
}

export type ComposerInterceptOutcome = { kind: "send"; text: string } | { kind: "cancel" };

/** Runs interceptors in order. Each one sees the text returned by the previous one. */
export async function runComposerInterceptors(
  interceptors: readonly PluginComposerInterceptorContribution[],
  input: PluginComposerInterceptInput,
): Promise<ComposerInterceptOutcome> {
  let text = input.text;
  for (const interceptor of interceptors) {
    const result = await interceptor.intercept({ ...input, target: { ...input.target }, text });
    if (!result) continue;
    if ("cancel" in result && result.cancel) return { kind: "cancel" };
    if ("text" in result && typeof result.text === "string") text = result.text;
  }
  return { kind: "send", text };
}
