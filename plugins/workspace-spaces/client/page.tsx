import { useEffect, useRef, useState } from "react";
import { PanResponder, Pressable, ScrollView, Text, TextInput, View } from "react-native";
import { useRpc, useSettings, type PluginSurfaceProps } from "@getpaseo/plugin/client";
import { SettingsSelect } from "@getpaseo/plugin/client/ui";
import { useQuery } from "@tanstack/react-query";
import {
  addSpace,
  adjacent,
  catalogRpc,
  membership,
  moveProject,
  preferences,
  projectKey,
  renameSpace,
} from "../shared/spaces";
import { bindWheel } from "./web";
import { touchDirection } from "./gesture";

// Page navigation preserves selection per host without sharing it with other clients.
const selections = new Map<string, string>();
export function SpacesPage(props: PluginSurfaceProps) {
  return <HostPage key={props.host.id} {...props} />;
}
function HostPage({ host, theme, navigation, layout }: PluginSurfaceProps) {
  const settings = useSettings(preferences);
  const read = useRpc(catalogRpc);
  const catalog = useQuery({
    queryKey: ["workspace-spaces", host.id],
    queryFn: () => read({}),
    retry: false,
    refetchInterval: 15000,
  });
  const [active, setActive] = useState(selections.get(host.id) ?? "space-1");
  const [moving, setMoving] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const region = useRef<View>(null);
  const scroll = useRef<ScrollView>(null);
  const positions = useRef(new Map<string, number>());
  const values = settings.status === "ready" ? settings.values : null;
  const selected = values?.spaces.some((s) => s.id === active)
    ? active
    : (values?.spaces[0].id ?? "space-1");
  const select = (id: string) => {
    setActive(id);
    selections.set(host.id, id);
    setMoving(null);
    setRenaming(null);
    setNotice("");
  };
  const switchRef = useRef((_direction: number) => {});
  switchRef.current = (direction) => {
    if (values)
      select(
        adjacent(
          values.spaces.map((s) => s.id),
          selected,
          direction,
        ),
      );
  };
  const ready = settings.status === "ready";
  useEffect(() => bindWheel(region.current, (direction) => switchRef.current(direction)), [ready]);
  useEffect(() => {
    scroll.current?.scrollTo({ y: positions.current.get(selected) ?? 0, animated: false });
  }, [selected]);
  const pan = useRef(
    PanResponder.create({
      onMoveShouldSetPanResponder: (_, g) =>
        Math.abs(g.dx) > 15 && Math.abs(g.dx) > Math.abs(g.dy) * 1.5,
      onPanResponderRelease: (_, g) => {
        const direction = touchDirection(g.dx, g.dy);
        if (direction) switchRef.current(direction);
      },
      onPanResponderTerminationRequest: () => true,
    }),
  ).current;
  const color = { color: theme.colors.foreground };
  const button = { padding: 12, borderRadius: 8, backgroundColor: theme.colors.surface1 };
  async function create() {
    if (settings.status !== "ready" || pending.current) return;
    pending.current = true;
    setBusy(true);
    try {
      const next = addSpace(settings.values);
      if (await settings.save(next, settings.revision))
        select(next.spaces[next.spaces.length - 1].id);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not create workspace.");
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  async function rename() {
    if (settings.status !== "ready" || pending.current || !renaming) return;
    pending.current = true;
    setBusy(true);
    try {
      const next = renameSpace(settings.values, renaming.id, renaming.name);
      if (await settings.save(next, settings.revision)) setRenaming(null);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not rename workspace.");
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  async function move(projectId: string, target: string) {
    if (settings.status !== "ready" || pending.current) return;
    if (!catalog.data?.projects.some((p) => p.id === projectId)) {
      setNotice("Project unavailable. Refresh and try again.");
      return;
    }
    pending.current = true;
    setBusy(true);
    try {
      const next = moveProject(settings.values, projectKey(host.id, projectId), target);
      if (await settings.save(next, settings.revision)) {
        setMoving(null);
        setNotice("Project moved.");
      }
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not move project.");
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  const refresh = () => {
    void settings.reload();
    void catalog.refetch();
  };
  const projects =
    catalog.data?.projects.filter(
      (p) => values && membership(values, projectKey(host.id, p.id)) === selected,
    ) ?? [];
  return (
    <View
      style={{
        flex: 1,
        backgroundColor: theme.colors.surface0,
        padding: layout.compact ? 12 : 24,
        gap: 12,
      }}
    >
      <View
        style={{
          flexDirection: "row",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 12,
        }}
      >
        <View style={{ flex: 1 }}>
          <Text style={{ ...color, fontSize: 24, fontWeight: "600" }}>Spaces</Text>
          <Text style={{ color: theme.colors.foregroundMuted }}>
            {host.label} · {values?.spaces.find((s) => s.id === selected)?.name ?? "Loading"}
          </Text>
        </View>
        <Pressable accessibilityRole="button" onPress={refresh} disabled={busy} style={button}>
          <Text style={color}>Refresh</Text>
        </Pressable>
      </View>
      {settings.status !== "ready" ? (
        <Text accessibilityRole="alert" style={color}>
          {settings.status === "loading"
            ? "Loading Spaces…"
            : "Cannot load Spaces. Refresh to retry; saved data is preserved."}
        </Text>
      ) : (
        <>
          {settings.saveError ? (
            <Text accessibilityRole="alert" style={color}>
              {settings.saveError} Refresh before retrying.
            </Text>
          ) : null}
          {notice ? (
            <Text accessibilityRole="alert" style={color}>
              {notice}
            </Text>
          ) : null}
          {renaming ? (
            <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 8 }}>
              <TextInput
                accessibilityLabel="Workspace name"
                autoFocus
                selectTextOnFocus
                value={renaming.name}
                editable={!busy}
                onChangeText={(name) => setRenaming({ ...renaming, name })}
                onSubmitEditing={() => {
                  void rename();
                }}
                style={{ ...color, ...button, flexGrow: 1, minWidth: 120 }}
              />
              <Pressable
                accessibilityRole="button"
                disabled={busy || !renaming.name.trim()}
                onPress={() => {
                  void rename();
                }}
                style={button}
              >
                <Text style={color}>Save</Text>
              </Pressable>
              <Pressable
                accessibilityRole="button"
                disabled={busy}
                onPress={() => setRenaming(null)}
                style={button}
              >
                <Text style={color}>Cancel</Text>
              </Pressable>
            </View>
          ) : (
            <Pressable
              accessibilityRole="button"
              disabled={busy || settings.saving}
              onPress={() => {
                const space = settings.values.spaces.find((s) => s.id === selected);
                if (space) {
                  setNotice("");
                  setRenaming({ ...space });
                }
              }}
              style={{ ...button, alignSelf: "flex-start" }}
            >
              <Text style={color}>Rename workspace</Text>
            </Pressable>
          )}
          <View ref={region} style={{ flex: 1, minHeight: 100 }} {...pan.panHandlers}>
            <ScrollView
              ref={scroll}
              onScroll={(e) => positions.current.set(selected, e.nativeEvent.contentOffset.y)}
              scrollEventThrottle={32}
              contentContainerStyle={{ gap: 12, paddingBottom: 12 }}
            >
              {catalog.isPending ? (
                <Text style={color}>Loading projects…</Text>
              ) : catalog.isError ? (
                <Text accessibilityRole="alert" style={color}>
                  Cannot refresh projects. Check the host connection and retry.
                </Text>
              ) : null}
              {!catalog.isPending && !catalog.isError && projects.length === 0 ? (
                <View style={{ padding: 24, gap: 12 }}>
                  <Text style={color}>No projects in this workspace.</Text>
                  <Text style={{ color: theme.colors.foregroundMuted }}>
                    Open another workspace and use “Move to workspace” on a project.
                  </Text>
                  {selected !== settings.values.spaces[0].id ? (
                    <Pressable
                      accessibilityRole="button"
                      style={button}
                      onPress={() => select(settings.values.spaces[0].id)}
                    >
                      <Text style={color}>Open {settings.values.spaces[0].name}</Text>
                    </Pressable>
                  ) : null}
                </View>
              ) : null}
              {projects.map((project) => (
                <View
                  key={project.id}
                  style={{
                    padding: 16,
                    borderRadius: 12,
                    backgroundColor: theme.colors.surface1,
                    gap: 8,
                  }}
                >
                  <View
                    style={{
                      flexDirection: "row",
                      justifyContent: "space-between",
                      gap: 8,
                      flexWrap: "wrap",
                    }}
                  >
                    <Text style={{ ...color, fontSize: 18, fontWeight: "600" }}>
                      {project.name}
                    </Text>
                    <Pressable
                      accessibilityRole="button"
                      accessibilityLabel={`Move ${project.name} to workspace`}
                      disabled={busy || catalog.isError}
                      onPress={() => setMoving(moving === project.id ? null : project.id)}
                      style={{ padding: 6 }}
                    >
                      <Text style={color}>Move to workspace</Text>
                    </Pressable>
                  </View>
                  <Text numberOfLines={1} style={{ color: theme.colors.foregroundMuted }}>
                    {project.path}
                  </Text>
                  {moving === project.id ? (
                    <SettingsSelect
                      label={`Move ${project.name}`}
                      value={selected}
                      options={settings.values.spaces.map((s) => ({ label: s.name, value: s.id }))}
                      disabled={busy || catalog.isError}
                      onValueChange={(id) => {
                        void move(project.id, id);
                      }}
                    />
                  ) : null}
                  {project.workspaces.length === 0 ? (
                    <Text style={{ color: theme.colors.foregroundMuted }}>
                      No active coding workspaces.
                    </Text>
                  ) : (
                    project.workspaces.map((workspace) => (
                      <Pressable
                        key={workspace.id}
                        accessibilityRole="button"
                        disabled={!navigation?.openWorkspace}
                        onPress={() => navigation?.openWorkspace({ workspaceId: workspace.id })}
                        style={{ paddingVertical: 10 }}
                      >
                        <Text style={color}>{workspace.name} →</Text>
                      </Pressable>
                    ))
                  )}
                </View>
              ))}
            </ScrollView>
          </View>
          <Text style={{ color: theme.colors.foregroundMuted }}>
            Swipe left or right across the project list to switch.
          </Text>
          <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
            <ScrollView horizontal style={{ flex: 1 }} contentContainerStyle={{ gap: 8 }}>
              {settings.values.spaces.map((space, index) => (
                <Pressable
                  key={space.id}
                  accessibilityRole="tab"
                  accessibilityLabel={space.name}
                  accessibilityState={{ selected: selected === space.id }}
                  onPress={() => select(space.id)}
                  style={{
                    ...button,
                    minWidth: 44,
                    alignItems: "center",
                    borderWidth: 2,
                    borderColor: selected === space.id ? theme.colors.foreground : "transparent",
                  }}
                >
                  <Text style={color}>{index + 1}</Text>
                </Pressable>
              ))}
            </ScrollView>
            <Pressable
              accessibilityRole="button"
              accessibilityLabel="Create workspace tab"
              disabled={busy || settings.saving}
              onPress={() => {
                void create();
              }}
              style={button}
            >
              <Text style={color}>+ New</Text>
            </Pressable>
          </View>
        </>
      )}
    </View>
  );
}
