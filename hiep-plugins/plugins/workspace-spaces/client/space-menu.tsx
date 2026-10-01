import type { PluginSidebarSectionProps } from "@getpaseo/plugin/client";
import { useState } from "react";
import {
  Modal,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  View,
  useWindowDimensions,
} from "react-native";

type Colors = PluginSidebarSectionProps["theme"]["colors"];
export type MenuAnchor = { x: number; y: number; width: number; height: number };
type Page = "menu" | "rename" | "remove" | "projects" | "target";
type SpaceOption = { id: string; name: string };
type ProjectOption = { id: string; name: string; spaceId: string };

const WIDTH = 220;
const GAP = 6;
const EDGE = 8;

/**
 * A Space tab's menu, floating above the tab like the host's menus. A press outside or Escape
 * closes it. Remove asks for confirmation and names the Space that receives the projects. Move
 * projects lists every project with its Space; picking one chooses where it goes.
 */
export function SpaceMenu(props: {
  anchor: MenuAnchor;
  colors: Colors;
  spaceId: string;
  name: string;
  spaces: readonly SpaceOption[];
  projects: readonly ProjectOption[];
  onMove: (projectId: string, spaceId: string) => Promise<boolean>;
  busy: boolean;
  /** Name of the Space that receives the projects, or null when this is the last Space. */
  removeTarget: string | null;
  onClose: () => void;
  onRename: (name: string) => Promise<boolean>;
  onRemove: () => Promise<boolean>;
}) {
  const { anchor, colors, spaceId, name, spaces, projects, busy, removeTarget } = props;
  const { onClose, onRename, onRemove, onMove } = props;
  const [movingId, setMovingId] = useState<string | null>(null);
  const moving = projects.find((project) => project.id === movingId);
  const spaceName = (id: string) => spaces.find((space) => space.id === id)?.name ?? "";
  // This Space's projects first, then the rest, each group by name.
  const ordered = [...projects].sort(
    (a, b) =>
      Number(b.spaceId === spaceId) - Number(a.spaceId === spaceId) || a.name.localeCompare(b.name),
  );
  const here = projects.filter((project) => project.spaceId === spaceId).length;
  const window = useWindowDimensions();
  const [page, setPage] = useState<Page>("menu");
  const [draft, setDraft] = useState(name);
  const left = Math.max(EDGE, Math.min(anchor.x, window.width - WIDTH - EDGE));
  const bottom = window.height - anchor.y + GAP;
  const saveName = () => {
    if (!draft.trim() || busy) return;
    void onRename(draft).then((saved) => saved && onClose());
  };
  return (
    <Modal transparent visible animationType="none" onRequestClose={onClose}>
      <Pressable
        accessibilityLabel="Close menu"
        onPress={onClose}
        style={{ position: "absolute", top: 0, right: 0, bottom: 0, left: 0 }}
      />
      <View
        accessibilityRole="menu"
        style={{
          position: "absolute",
          left,
          bottom,
          width: WIDTH,
          padding: 4,
          gap: 2,
          borderWidth: 1,
          borderColor: colors.border,
          borderRadius: 10,
          backgroundColor: colors.surface0,
          shadowColor: "#000",
          shadowOpacity: 0.12,
          shadowRadius: 16,
          shadowOffset: { width: 0, height: 6 },
          elevation: 8,
        }}
      >
        <Text
          numberOfLines={1}
          style={{
            color: colors.foregroundMuted,
            fontSize: 11,
            paddingHorizontal: 8,
            paddingTop: 4,
            paddingBottom: 2,
          }}
        >
          {page === "target" && moving ? `Move ${moving.name} to` : name}
        </Text>
        {page === "projects" || page === "target" ? (
          <MenuRow
            label="‹ Back"
            colors={colors}
            onPress={() => setPage(page === "target" ? "projects" : "menu")}
          />
        ) : null}
        {page === "projects" ? (
          <ScrollView style={{ maxHeight: 280 }}>
            {ordered.map((project) => (
              <MenuRow
                key={project.id}
                label={project.name}
                trailing={spaceName(project.spaceId)}
                colors={colors}
                disabled={busy}
                onPress={() => {
                  setMovingId(project.id);
                  setPage("target");
                }}
              />
            ))}
          </ScrollView>
        ) : null}
        {page === "target" && moving ? (
          <ScrollView style={{ maxHeight: 280 }}>
            {spaces.map((space) => (
              <MenuRow
                key={space.id}
                label={space.name}
                checked={space.id === moving.spaceId}
                colors={colors}
                disabled={busy}
                onPress={() => {
                  if (space.id === moving.spaceId) return setPage("projects");
                  void onMove(moving.id, space.id).then((moved) => moved && setPage("projects"));
                }}
              />
            ))}
          </ScrollView>
        ) : null}
        {page === "menu" ? (
          <>
            <MenuRow
              label="Rename…"
              colors={colors}
              disabled={busy}
              onPress={() => {
                setDraft(name);
                setPage("rename");
              }}
            />
            <MenuRow
              label="Move projects…"
              trailing={String(here)}
              colors={colors}
              disabled={projects.length === 0}
              onPress={() => setPage("projects")}
            />
            <View style={{ height: 1, marginVertical: 2, backgroundColor: colors.border }} />
            <MenuRow
              label={removeTarget ? "Remove…" : "Keep at least one Space"}
              colors={colors}
              danger={removeTarget !== null}
              disabled={removeTarget === null || busy}
              onPress={() => setPage("remove")}
            />
          </>
        ) : null}
        {page === "rename" ? (
          <View style={{ gap: 6, padding: 4 }}>
            <TextInput
              autoFocus
              selectTextOnFocus
              accessibilityLabel="Space name"
              value={draft}
              onChangeText={setDraft}
              onSubmitEditing={saveName}
              style={{
                height: 28,
                paddingHorizontal: 8,
                fontSize: 12,
                color: colors.foreground,
                borderWidth: 1,
                borderColor: colors.border,
                borderRadius: 6,
              }}
            />
            <Actions colors={colors}>
              <ActionButton label="Cancel" colors={colors} onPress={() => setPage("menu")} />
              <ActionButton
                label="Save"
                colors={colors}
                tone="primary"
                disabled={busy || !draft.trim()}
                onPress={saveName}
              />
            </Actions>
          </View>
        ) : null}
        {page === "remove" && removeTarget ? (
          <View style={{ gap: 8, padding: 4 }}>
            <Text style={{ fontSize: 12, color: colors.foreground }}>
              Remove “{name}”? Its projects move to {removeTarget}.
            </Text>
            <Actions colors={colors}>
              <ActionButton label="Cancel" colors={colors} onPress={() => setPage("menu")} />
              <ActionButton
                label="Remove"
                colors={colors}
                tone="danger"
                disabled={busy}
                onPress={() => void onRemove().then((removed) => removed && onClose())}
              />
            </Actions>
          </View>
        ) : null}
      </View>
    </Modal>
  );
}

function MenuRow(props: {
  label: string;
  colors: Colors;
  onPress: () => void;
  disabled?: boolean;
  danger?: boolean;
  /** Muted value at the trailing edge, such as a project's current Space. */
  trailing?: string;
  checked?: boolean;
}) {
  const { label, colors, onPress, disabled, danger, trailing, checked } = props;
  // Hover fires on web only; native rows tint while pressed.
  const [hovered, setHovered] = useState(false);
  return (
    <Pressable
      accessibilityRole="menuitem"
      accessibilityLabel={label}
      disabled={disabled}
      onPress={onPress}
      onHoverIn={() => setHovered(true)}
      onHoverOut={() => setHovered(false)}
      style={({ pressed }) => ({
        minHeight: 28,
        paddingHorizontal: 8,
        borderRadius: 6,
        flexDirection: "row",
        alignItems: "center",
        gap: 8,
        opacity: disabled ? 0.5 : 1,
        backgroundColor: !disabled && (hovered || pressed) ? colors.surface2 : "transparent",
      })}
    >
      <Text
        numberOfLines={1}
        style={{ flex: 1, fontSize: 12, color: danger ? colors.statusDanger : colors.foreground }}
      >
        {label}
      </Text>
      {trailing ? (
        <Text
          numberOfLines={1}
          style={{ maxWidth: 90, fontSize: 11, color: colors.foregroundMuted }}
        >
          {trailing}
        </Text>
      ) : null}
      {checked ? <Text style={{ fontSize: 12, color: colors.foregroundMuted }}>✓</Text> : null}
    </Pressable>
  );
}

function Actions({ children }: { colors: Colors; children: React.ReactNode }) {
  return (
    <View style={{ flexDirection: "row", justifyContent: "flex-end", gap: 6 }}>{children}</View>
  );
}

function ActionButton(props: {
  label: string;
  colors: Colors;
  onPress: () => void;
  disabled?: boolean;
  tone?: "primary" | "danger";
}) {
  const { label, colors, onPress, disabled, tone } = props;
  const fill = tone === "primary" ? colors.accent : tone === "danger" ? colors.statusDanger : null;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      disabled={disabled}
      onPress={onPress}
      style={{
        height: 26,
        paddingHorizontal: 10,
        borderRadius: 6,
        justifyContent: "center",
        opacity: disabled ? 0.5 : 1,
        backgroundColor: fill ?? "transparent",
        borderWidth: fill ? 0 : 1,
        borderColor: colors.border,
      }}
    >
      <Text style={{ fontSize: 12, color: fill ? colors.accentForeground : colors.foreground }}>
        {label}
      </Text>
    </Pressable>
  );
}
