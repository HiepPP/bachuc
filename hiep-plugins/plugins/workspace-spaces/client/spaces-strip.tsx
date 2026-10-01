import type { PluginSidebarSectionProps } from "@getpaseo/plugin/client";
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { Pressable, Text, View } from "react-native";
import { removalTarget } from "../shared/spaces";
import type { SidebarController } from "./sidebar-state";
import { SpaceMenu, type MenuAnchor } from "./space-menu";
import type { SpacesSidebar } from "./spaces-sidebar";

// React Native Web forwards onContextMenu to the DOM; native clients use long-press instead.
function contextMenuProps(open: () => void): object {
  return {
    onContextMenu: (event: { preventDefault?: () => void }) => {
      event.preventDefault?.();
      open();
    },
  };
}

/** Numbered Space tabs above the sidebar footer. A tab's menu renames or removes its Space. */
export function createSpacesStrip(
  controller: SidebarController,
  sidebar: SpacesSidebar,
  setActiveHost: (host: string | null) => void,
) {
  let version = 0;
  const subscribe = (listener: () => void) =>
    sidebar.subscribe(() => {
      version++;
      listener();
    });
  return function SpacesStrip({ theme, activeServerId }: PluginSidebarSectionProps) {
    useSyncExternalStore(subscribe, () => version);
    // The menu stays closed until a tab is right-clicked or long-pressed.
    const [menu, setMenu] = useState<{ spaceId: string; anchor: MenuAnchor } | null>(null);
    const tabs = useRef(new Map<string, View>());
    useEffect(() => {
      setActiveHost(activeServerId);
      void controller.refresh();
    }, [activeServerId]);
    const snapshot = controller.get();
    const { colors } = theme;
    if (!snapshot) {
      const error = controller.getLoadError();
      return error ? (
        <Pressable onPress={() => void controller.refresh()} style={{ padding: 8 }}>
          <Text style={{ color: colors.foregroundMuted, fontSize: 12 }}>
            Spaces unavailable: {error}. Tap to retry.
          </Text>
        </Pressable>
      ) : null;
    }
    const selected = sidebar.selectedSpace(activeServerId);
    const menuSpace = menu
      ? snapshot.state.spaces.find((space) => space.id === menu.spaceId)
      : undefined;
    const removeTarget =
      menuSpace && snapshot.state.spaces.length > 1
        ? (snapshot.state.spaces.find(
            (space) => space.id === removalTarget(snapshot.state, menuSpace.id),
          )?.name ?? null)
        : null;
    const openMenu = (spaceId: string) => {
      sidebar.select(activeServerId, spaceId);
      tabs.current.get(spaceId)?.measureInWindow((x, y, width, height) => {
        setMenu({ spaceId, anchor: { x, y, width, height } });
      });
    };
    const tab = (active: boolean) => ({
      minWidth: 26,
      height: 26,
      paddingHorizontal: 7,
      borderRadius: 6,
      alignItems: "center" as const,
      justifyContent: "center" as const,
      borderWidth: 1,
      borderColor: active ? colors.accent : colors.border,
      backgroundColor: active ? colors.accent : "transparent",
    });
    return (
      <View
        style={{
          gap: 6,
          paddingHorizontal: 10,
          paddingVertical: 6,
          borderTopWidth: 1,
          borderColor: colors.border,
        }}
      >
        <View style={{ flexDirection: "row", gap: 4, flexWrap: "wrap", alignItems: "center" }}>
          {snapshot.state.spaces.map((space, index) => (
            <Pressable
              key={space.id}
              ref={(node) => {
                if (node) tabs.current.set(space.id, node);
                else tabs.current.delete(space.id);
              }}
              accessibilityRole="tab"
              accessibilityLabel={space.name}
              accessibilityState={{ selected: space.id === selected }}
              accessibilityHint="Right-click or long-press for Rename and Remove"
              onPress={() => sidebar.select(activeServerId, space.id)}
              onLongPress={() => openMenu(space.id)}
              {...contextMenuProps(() => openMenu(space.id))}
              style={tab(space.id === selected)}
            >
              <Text
                style={{
                  fontSize: 12,
                  color: space.id === selected ? colors.accentForeground : colors.foreground,
                }}
              >
                {index + 1}
              </Text>
            </Pressable>
          ))}
          <Pressable
            accessibilityLabel="Create a Space"
            disabled={snapshot.busy}
            onPress={() => void sidebar.create(activeServerId)}
            style={tab(false)}
          >
            <Text style={{ fontSize: 12, color: colors.foreground }}>+</Text>
          </Pressable>
        </View>
        {snapshot.error ? (
          <Text style={{ color: colors.statusDanger, fontSize: 12 }}>{snapshot.error}</Text>
        ) : null}
        {menu && menuSpace ? (
          <SpaceMenu
            key={menuSpace.id}
            anchor={menu.anchor}
            colors={colors}
            spaceId={menuSpace.id}
            name={menuSpace.name}
            spaces={snapshot.state.spaces}
            projects={sidebar.projects(activeServerId)}
            onMove={(projectId, spaceId) => sidebar.moveProject(activeServerId, projectId, spaceId)}
            busy={snapshot.busy}
            removeTarget={removeTarget}
            onClose={() => setMenu(null)}
            onRename={(name) => controller.rename(menuSpace.id, name)}
            onRemove={() => sidebar.remove(activeServerId, menuSpace.id)}
          />
        ) : null}
      </View>
    );
  };
}
