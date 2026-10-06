import { create } from "zustand";

export interface NewWorkspaceSidePanelTarget {
  serverId: string;
  projectId: string;
  cwd: string;
}

interface NewWorkspaceSidePanelState {
  /** The project chosen on the mounted new workspace screen, or null outside that screen. */
  target: NewWorkspaceSidePanelTarget | null;
  /** Null until the user shows or hides the panel on this screen; the setting decides until then. */
  open: boolean | null;
  /** The `pluginId/panelId` key of the shown panel; null shows the first panel. */
  selected: string | null;
  setTarget: (target: NewWorkspaceSidePanelTarget | null) => void;
  show: (selected?: string) => void;
  hide: () => void;
  reset: () => void;
}

export function newWorkspacePanelKey(pluginId: string, panelId: string): string {
  return `${pluginId}/${panelId}`;
}

export const useNewWorkspaceSidePanelStore = create<NewWorkspaceSidePanelState>((set) => ({
  target: null,
  open: null,
  selected: null,
  setTarget: (target) => set({ target }),
  show: (selected) => set((state) => ({ open: true, selected: selected ?? state.selected })),
  hide: () => set({ open: false }),
  reset: () => set({ target: null, open: null, selected: null }),
}));
