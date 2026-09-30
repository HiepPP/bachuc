import AsyncStorage from "@react-native-async-storage/async-storage";
import { create } from "zustand";
import { persist, type StateStorage } from "zustand/middleware";
import { z } from "zod";
import { createValidatedPersistStorage } from "@/storage/validated-persist-storage";

const ACTIVE_HOST_STORAGE_KEY = "active-host";
const ACTIVE_HOST_STORE_VERSION = 1;

interface ActiveHostPersistedState {
  // null means "All machines": every screen shows every host.
  activeServerId: string | null;
  lastWorkspaceIdByServerId: Record<string, string>;
  /** Last host-scoped pathname seen per host, so switching back lands on the same screen. */
  lastRouteByServerId: Record<string, string>;
}

interface ActiveHostStoreState extends ActiveHostPersistedState {
  /** The host that was active before the current one. Session-only, never persisted. */
  previousServerId: string | null;
  setActiveServerId: (serverId: string | null) => void;
  rememberWorkspace: (serverId: string, workspaceId: string) => void;
  rememberRoute: (serverId: string, pathname: string) => void;
  reconcile: (serverIds: readonly string[]) => void;
}

const ActiveHostPersistedStateSchema: z.ZodType<ActiveHostPersistedState> = z.strictObject({
  activeServerId: z.string().min(1).nullable(),
  lastWorkspaceIdByServerId: z.record(z.string(), z.string()),
  // Optional so state saved before this field existed still loads.
  lastRouteByServerId: z.record(z.string(), z.string()).default({}),
});

// A storage error must still finish hydration: startup redirects wait for it.
const activeHostStorage: StateStorage = {
  getItem: async (name) => {
    try {
      return await AsyncStorage.getItem(name);
    } catch {
      return null;
    }
  },
  setItem: async (name, value) => {
    try {
      await AsyncStorage.setItem(name, value);
    } catch {}
  },
  removeItem: async (name) => {
    try {
      await AsyncStorage.removeItem(name);
    } catch {}
  },
};

/** The active machine only counts while it is still a known host; otherwise it reads as "All". */
export function resolveActiveServerId(
  activeServerId: string | null,
  serverIds: readonly string[],
): string | null {
  return activeServerId !== null && serverIds.includes(activeServerId) ? activeServerId : null;
}

/** The host after the active one, wrapping around. From "All hosts" it starts at the first. */
export function getNextActiveServerId(
  orderedServerIds: readonly string[],
  activeServerId: string | null,
): string | null {
  if (orderedServerIds.length === 0) return null;
  const index = activeServerId === null ? -1 : orderedServerIds.indexOf(activeServerId);
  return orderedServerIds[(index + 1) % orderedServerIds.length] ?? null;
}

export const useActiveHostStore = create<ActiveHostStoreState>()(
  persist(
    (set) => ({
      activeServerId: null,
      previousServerId: null,
      lastWorkspaceIdByServerId: {},
      lastRouteByServerId: {},
      setActiveServerId: (serverId) =>
        set((state) => {
          if (state.activeServerId === serverId) return state;
          return {
            activeServerId: serverId,
            previousServerId: state.activeServerId ?? state.previousServerId,
          };
        }),
      rememberWorkspace: (serverId, workspaceId) =>
        set((state) =>
          state.lastWorkspaceIdByServerId[serverId] === workspaceId
            ? state
            : {
                lastWorkspaceIdByServerId: {
                  ...state.lastWorkspaceIdByServerId,
                  [serverId]: workspaceId,
                },
              },
        ),
      rememberRoute: (serverId, pathname) =>
        set((state) =>
          state.lastRouteByServerId[serverId] === pathname
            ? state
            : { lastRouteByServerId: { ...state.lastRouteByServerId, [serverId]: pathname } },
        ),
      reconcile: (serverIds) =>
        set((state) => {
          const allowed = new Set(serverIds);
          const activeServerId =
            state.activeServerId !== null && allowed.has(state.activeServerId)
              ? state.activeServerId
              : null;
          const previousServerId =
            state.previousServerId !== null && allowed.has(state.previousServerId)
              ? state.previousServerId
              : null;
          const workspaces = Object.entries(state.lastWorkspaceIdByServerId);
          const keptWorkspaces = workspaces.filter(([serverId]) => allowed.has(serverId));
          const routes = Object.entries(state.lastRouteByServerId);
          const keptRoutes = routes.filter(([serverId]) => allowed.has(serverId));
          if (
            activeServerId === state.activeServerId &&
            previousServerId === state.previousServerId &&
            keptWorkspaces.length === workspaces.length &&
            keptRoutes.length === routes.length
          ) {
            return state;
          }
          return {
            activeServerId,
            previousServerId,
            lastWorkspaceIdByServerId: Object.fromEntries(keptWorkspaces),
            lastRouteByServerId: Object.fromEntries(keptRoutes),
          };
        }),
    }),
    {
      name: ACTIVE_HOST_STORAGE_KEY,
      version: ACTIVE_HOST_STORE_VERSION,
      storage: createValidatedPersistStorage(activeHostStorage, ActiveHostPersistedStateSchema),
      partialize: (state) => ({
        activeServerId: state.activeServerId,
        lastWorkspaceIdByServerId: state.lastWorkspaceIdByServerId,
        lastRouteByServerId: state.lastRouteByServerId,
      }),
    },
  ),
);
