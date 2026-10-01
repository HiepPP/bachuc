import { settingsRpc } from "@getpaseo/plugin";
import type { PluginClientContext } from "@getpaseo/plugin/client";
import {
  addSpace,
  removeSpace,
  renameSpace,
  catalogRpc,
  hostState,
  moveProject,
  preferencesSchema,
  projectKey,
  withHostState,
  type SpacesPreferences,
  type SpacesState,
} from "../shared/spaces";

export type SidebarProject = {
  id: string;
  name: string;
  viewKey?: string;
  workspaces: { id: string; name: string }[];
};
export type SidebarSnapshot = {
  /** Active host's serverId, or null for "All hosts". */
  host: string | null;
  state: SpacesState;
  projects: SidebarProject[];
  busy: boolean;
  error: string;
};
export function sidebarMembership(state: SpacesState, id: string): string {
  const canonical = projectKey("sidebar", id);
  if (Object.hasOwn(state.members, canonical)) return state.members[canonical];
  for (const [key, value] of Object.entries(state.members)) {
    try {
      const pair = JSON.parse(key);
      if (Array.isArray(pair) && pair[1] === id) return value;
    } catch {
      /* Unknown legacy keys stay untouched. */
    }
  }
  return state.spaces[0].id;
}
export function viewMembership(state: SpacesState, key: string, project?: SidebarProject): string {
  const stored = projectKey("view", key);
  return Object.hasOwn(state.members, stored)
    ? state.members[stored]
    : project
      ? sidebarMembership(state, project.id)
      : state.spaces[0].id;
}
export function matchProject(key: string, projects: SidebarProject[]) {
  const equivalent = projects.find((p) => p.viewKey === key);
  if (equivalent) return equivalent;
  try {
    const pair = JSON.parse(key);
    if (Array.isArray(pair)) return projects.find((p) => p.id === pair[1]);
  } catch {
    /* Not a placement key. */
  }
  return undefined;
}
export function createSidebarController(
  client: Pick<PluginClientContext, "rpc">,
  activeHost: () => string | null = () => null,
) {
  const contract = settingsRpc("spaces");
  type Saved = Omit<SidebarSnapshot, "host" | "state"> & { prefs: SpacesPreferences };
  let saved: Saved | null = null;
  let view: { from: Saved; snapshot: SidebarSnapshot } | null = null;
  // Each read resolves the active host, so switching hosts shows that host's own Spaces.
  function get(): SidebarSnapshot | null {
    if (!saved) return null;
    const host = activeHost();
    if (view?.from !== saved || view.snapshot.host !== host) {
      const { prefs, ...rest } = saved;
      view = { from: saved, snapshot: { ...rest, host, state: hostState(prefs, host) } };
    }
    return view.snapshot;
  }
  let loadError = "";
  let revision = "",
    stopped = false,
    loading = false;
  const listeners = new Set<() => void>();
  const emit = () => {
    if (!stopped) for (const listener of listeners) listener();
  };
  async function refresh() {
    if (stopped || loading || saved?.busy) return;
    loading = true;
    try {
      const [read, catalog] = await Promise.all([
        client.rpc(contract.read, {}),
        client.rpc(catalogRpc, {}),
      ]);
      if (stopped) return;
      if (read.status !== "ready") throw new Error(read.error);
      revision = read.revision;
      loadError = "";
      saved = {
        prefs: preferencesSchema.parse(read.values),
        projects: catalog.projects,
        busy: false,
        error: "",
      };
    } catch (e) {
      loadError = e instanceof Error ? e.message : "Host unavailable";
      if (saved) saved = { ...saved, error: loadError };
    } finally {
      loading = false;
      emit();
    }
  }
  async function save(change: (s: SpacesState) => SpacesState) {
    if (!saved || saved.busy || loading || stopped) return false;
    const host = activeHost();
    saved = { ...saved, busy: true, error: "" };
    emit();
    try {
      const values = withHostState(saved.prefs, host, change(hostState(saved.prefs, host)));
      const result = await client.rpc(contract.write, { revision, values });
      if (stopped) return false;
      if (result.status !== "saved") throw new Error(result.error);
      revision = result.revision;
      saved = { ...saved, prefs: preferencesSchema.parse(result.values), busy: false };
      emit();
      return true;
    } catch (e) {
      if (!stopped) {
        saved = {
          ...saved,
          busy: false,
          error: e instanceof Error ? e.message : "Save failed",
        };
        emit();
      }
      return false;
    }
  }
  return {
    get,
    getLoadError: () => loadError,
    // The active host changed; readers recompute that host's Spaces.
    notify: emit,
    subscribe(fn: () => void) {
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    },
    refresh,
    create: () => save(addSpace),
    rename: (id: string, name: string) => save((state) => renameSpace(state, id, name)),
    remove: (id: string) => save((state) => removeSpace(state, id)),
    move: (id: string, target: string) =>
      save((state) => {
        if (!saved?.projects.some((p) => p.id === id))
          throw new Error("Project unavailable. Refresh first.");
        const next = moveProject(state, projectKey("sidebar", id), target);
        // Keep membership saved by the original standalone page in sync.
        for (const key of Object.keys(next.members)) {
          try {
            if (JSON.parse(key)?.[1] === id) next.members[key] = target;
          } catch {
            /* Preserve unknown keys. */
          }
        }
        return next;
      }),
    moveView: (key: string, target: string) =>
      save((state) => {
        if (!key) throw new Error("Project unavailable. Reopen its menu.");
        const next = moveProject(state, projectKey("view", key), target);
        const project = matchProject(key, saved?.projects ?? []);
        if (project) {
          next.members[projectKey("sidebar", project.id)] = target;
          for (const member of Object.keys(next.members)) {
            try {
              if (JSON.parse(member)?.[1] === project.id) next.members[member] = target;
            } catch {
              /* Preserve legacy keys. */
            }
          }
        }
        return next;
      }),
    stop() {
      stopped = true;
      listeners.clear();
    },
  };
}
export type SidebarController = ReturnType<typeof createSidebarController>;
