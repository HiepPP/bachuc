import { defineRpc, defineSettings } from "@getpaseo/plugin";
import { z } from "zod";
const spacesShape = {
  spaces: z
    .array(z.object({ id: z.string().min(1), name: z.string().min(1) }))
    .min(1)
    .default([{ id: "space-1", name: "Workspace 1" }]),
  members: z.record(z.string(), z.string()).default({}),
};
const validSpaces = (s: { spaces: { id: string }[]; members: Record<string, string> }) =>
  new Set(s.spaces.map((x) => x.id)).size === s.spaces.length &&
  Object.values(s.members).every((id) => s.spaces.some((x) => x.id === id));
export const stateSchema = z.object(spacesShape).refine(validSpaces, "Invalid Space membership");
export type SpacesState = z.infer<typeof stateSchema>;
// Root spaces/members belong to the "All hosts" view; each pinned host keeps its own entry.
export const preferencesSchema = z
  .object({ ...spacesShape, hosts: z.record(z.string(), stateSchema).default({}) })
  .refine(validSpaces, "Invalid Space membership");
export type SpacesPreferences = z.infer<typeof preferencesSchema>;
export const preferences = defineSettings({
  id: "spaces",
  scope: "host",
  version: 1,
  schema: preferencesSchema,
});
export function hostState(prefs: SpacesPreferences, host: string | null): SpacesState {
  if (host === null) return { spaces: prefs.spaces, members: prefs.members };
  return Object.hasOwn(prefs.hosts, host) ? prefs.hosts[host] : stateSchema.parse({});
}
export function withHostState(
  prefs: SpacesPreferences,
  host: string | null,
  state: SpacesState,
): SpacesPreferences {
  return preferencesSchema.parse(
    host === null ? { ...prefs, ...state } : { ...prefs, hosts: { ...prefs.hosts, [host]: state } },
  );
}
export const catalogRpc = defineRpc({
  name: "spaces.catalog",
  input: z.object({}),
  output: z.object({
    projects: z.array(
      z.object({
        id: z.string(),
        name: z.string(),
        path: z.string(),
        viewKey: z.string().optional(),
        workspaces: z.array(z.object({ id: z.string(), name: z.string() })),
      }),
    ),
  }),
});
export const projectKey = (host: string, project: string) => JSON.stringify([host, project]);
export const membership = (state: SpacesState, key: string) =>
  Object.hasOwn(state.members, key) ? state.members[key] : state.spaces[0].id;
export function addSpace(state: SpacesState): SpacesState {
  const number = Math.max(...state.spaces.map((s) => Number(s.id.slice(6)) || 0)) + 1;
  return stateSchema.parse({
    ...state,
    spaces: [...state.spaces, { id: `space-${number}`, name: `Workspace ${number}` }],
  });
}
export function renameSpace(state: SpacesState, id: string, name: string): SpacesState {
  if (!state.spaces.some((space) => space.id === id))
    throw new Error("Workspace no longer exists. Refresh first.");
  const trimmed = name.trim();
  if (!trimmed) throw new Error("Enter a workspace name.");
  return stateSchema.parse({
    ...state,
    spaces: state.spaces.map((space) => (space.id === id ? { ...space, name: trimmed } : space)),
  });
}
export function moveProject(state: SpacesState, key: string, target: string): SpacesState {
  if (!state.spaces.some((s) => s.id === target))
    throw new Error("Workspace no longer exists. Refresh and try again.");
  return stateSchema.parse({ ...state, members: { ...state.members, [key]: target } });
}
export function adjacent(ids: string[], current: string, direction: number): string {
  const index = Math.max(0, ids.indexOf(current));
  return ids[Math.max(0, Math.min(ids.length - 1, index + direction))];
}

export function removalTarget(state: SpacesState, id: string): string {
  const index = state.spaces.findIndex((space) => space.id === id);
  if (index < 0) throw new Error("Workspace no longer exists. Refresh first.");
  if (state.spaces.length === 1) throw new Error("Keep at least one workspace.");
  return state.spaces[index > 0 ? index - 1 : 1].id;
}
export function removeSpace(state: SpacesState, id: string): SpacesState {
  const target = removalTarget(state, id);
  return stateSchema.parse({
    spaces: state.spaces.filter((space) => space.id !== id),
    members: Object.fromEntries(
      Object.entries(state.members).map(([key, value]) => [key, value === id ? target : value]),
    ),
  });
}
