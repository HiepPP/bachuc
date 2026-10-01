import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { Platform, View } from "react-native";
import { useReducedMotion } from "react-native-reanimated";
import { withUnistyles } from "react-native-unistyles";
import type { PluginTheme } from "@getpaseo/plugin";
import type {
  PluginSidebarFilterChange,
  PluginSidebarSectionContribution,
} from "@getpaseo/plugin/client";
import { useIsCompactFormFactor } from "@/constants/layout";
import { useActiveServerId } from "@/hosts/use-visible-hosts";
import type { SidebarProjectEntry } from "@/hooks/use-sidebar-workspaces-list";
import { useHostRuntimeClient, useHosts } from "@/runtime/host-runtime";
import type { Theme } from "@/styles/theme";
import { pluginRegistry } from "../registry";
import { PluginRuntimeBoundary } from "../runtime-boundary";
import { SurfaceErrorBoundary } from "../surface-error-boundary";
import { toPluginTheme } from "../theme";
import type { InstalledPlugin } from "../types";
import {
  hiddenProjectViewKeys,
  pluginProjectMenuItems,
  pluginSidebarTitle,
  selectSidebarPlugins,
  toPluginSidebarProject,
} from "./model";
import { slidePluginSidebar } from "./slide";

function useSidebarPlugins(): InstalledPlugin[] {
  const installed = useSyncExternalStore(
    pluginRegistry.subscribe,
    pluginRegistry.getSnapshot,
    pluginRegistry.getSnapshot,
  );
  const activeServerId = useActiveServerId();
  return useMemo(
    () => selectSidebarPlugins(installed, activeServerId),
    [activeServerId, installed],
  );
}

function subscribeToFilters(
  plugins: readonly InstalledPlugin[],
  listener: (change?: PluginSidebarFilterChange) => void,
) {
  const filters = plugins.flatMap((plugin) => plugin.sidebarProjectFilters ?? []);
  const unsubscribes = filters.map((filter) => filter.subscribe(listener));
  return () => {
    for (const unsubscribe of unsubscribes) unsubscribe();
  };
}

/** View keys hidden by plugin project filters; recomputed when a filter reports a change. */
export function usePluginHiddenProjectViewKeys(
  projects: readonly SidebarProjectEntry[],
): ReadonlySet<string> {
  // React Compiler would drop `revision` from the memo inputs, because the filters read plugin
  // state it cannot see. Opt out so a reported change re-runs them.
  "use no memo";
  const plugins = useSidebarPlugins();
  const activeServerId = useActiveServerId();
  const reduceMotion = useReducedMotion();
  const [revision, setRevision] = useState(0);
  useEffect(
    () =>
      subscribeToFilters(plugins, (change) => {
        const apply = () => setRevision(bumpRevision);
        if (change?.direction && !reduceMotion) slidePluginSidebar(change.direction, apply);
        else apply();
      }),
    [plugins, reduceMotion],
  );
  return useMemo(() => {
    void revision; // Filters answer from plugin state, so a reported change must re-run them.
    return hiddenProjectViewKeys(plugins, projects, { activeServerId });
  }, [activeServerId, plugins, projects, revision]);
}

function bumpRevision(value: number): number {
  return value + 1;
}

/** The first plugin filter title, which replaces the "Workspaces" heading; null keeps it. */
export function usePluginSidebarTitle(): string | null {
  // Titles answer from plugin state the compiler cannot see; see usePluginHiddenProjectViewKeys.
  "use no memo";
  const plugins = useSidebarPlugins();
  const activeServerId = useActiveServerId();
  const [revision, setRevision] = useState(0);
  useEffect(() => subscribeToFilters(plugins, () => setRevision(bumpRevision)), [plugins]);
  return useMemo(() => {
    void revision;
    return pluginSidebarTitle(plugins, { activeServerId });
  }, [activeServerId, plugins, revision]);
}

/** Calls each plugin filter's `onSwipe`, or null when no plugin handles swipes. */
export function usePluginSidebarSwipeHandler(): ((direction: 1 | -1) => void) | null {
  const plugins = useSidebarPlugins();
  const activeServerId = useActiveServerId();
  return useMemo(() => {
    const handlers = plugins.flatMap((plugin) =>
      (plugin.sidebarProjectFilters ?? []).flatMap((filter) =>
        filter.onSwipe ? [{ plugin, filter, onSwipe: filter.onSwipe }] : [],
      ),
    );
    if (handlers.length === 0) return null;
    return (direction) => {
      for (const { plugin, filter, onSwipe } of handlers) {
        try {
          onSwipe(direction, { activeServerId });
        } catch (error) {
          console.warn(`[Plugins] Sidebar swipe failed: ${plugin.id}/${filter.id}`, error);
        }
      }
    };
  }, [activeServerId, plugins]);
}

export function usePluginProjectMenuItems(project: SidebarProjectEntry | null) {
  const plugins = useSidebarPlugins();
  const activeServerId = useActiveServerId();
  return useMemo(
    () =>
      project
        ? pluginProjectMenuItems(plugins, toPluginSidebarProject(project), { activeServerId })
        : [],
    [activeServerId, plugins, project],
  );
}

function SectionRenderer({
  plugin,
  section,
  theme,
}: {
  plugin: InstalledPlugin;
  section: PluginSidebarSectionContribution;
  theme: PluginTheme;
}) {
  const client = useHostRuntimeClient(plugin.serverId);
  const hosts = useHosts();
  const activeServerId = useActiveServerId();
  const compact = useIsCompactFormFactor();
  const label = hosts.find((host) => host.serverId === plugin.serverId)?.label ?? plugin.serverId;
  const host = useMemo(() => ({ id: plugin.serverId, label }), [label, plugin.serverId]);
  const layout = useMemo(() => ({ compact, platform: resolvePlatform() }), [compact]);
  if (!client) return null;
  const Section = section.Component;
  return (
    <SurfaceErrorBoundary installation={plugin} Surface={Section}>
      <PluginRuntimeBoundary plugin={plugin} client={client}>
        <Section theme={theme} host={host} layout={layout} activeServerId={activeServerId} />
      </PluginRuntimeBoundary>
    </SurfaceErrorBoundary>
  );
}

const pluginThemeMapping = (theme: Theme) => ({ theme: toPluginTheme(theme) });
const ThemedSectionRenderer = withUnistyles(SectionRenderer);

function resolvePlatform(): "ios" | "android" | "web" {
  if (Platform.OS === "ios") return "ios";
  if (Platform.OS === "android") return "android";
  return "web";
}

/** Plugin sections, rendered above the sidebar footer. */
export function PluginSidebarSections() {
  const plugins = useSidebarPlugins();
  const sections = plugins.flatMap((plugin) =>
    (plugin.sidebarSections ?? []).map((section) => ({ plugin, section })),
  );
  if (sections.length === 0) return null;
  return (
    <View>
      {sections.map(({ plugin, section }) => (
        <ThemedSectionRenderer
          key={`${plugin.serverId}/${plugin.id}/${section.id}`}
          plugin={plugin}
          section={section}
          uniProps={pluginThemeMapping}
        />
      ))}
    </View>
  );
}
