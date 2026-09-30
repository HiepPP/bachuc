import { useSyncExternalStore } from "react";
import type { FileDiffRequest } from "../shared/turn-diff";

export type FileSelection = FileDiffRequest & { agentId: string };

// The panel API opens a panel by id only, so the clicked file travels through this store.
let selection: FileSelection | null = null;
const listeners = new Set<() => void>();

export function selectFile(next: FileSelection | null) {
  selection = next;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The file open in the Turn diff panel, so cards can mark it. */
export function useSelectedFile() {
  return useSyncExternalStore(subscribe, () => selection);
}
