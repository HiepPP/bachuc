import type { PluginTimelineItemProps } from "@getpaseo/plugin/client";
import { Icon } from "@getpaseo/plugin/client/react-native";
import { useMemo } from "react";
import { Pressable, Text, View } from "react-native";
import { imageType, type TurnDiff } from "../shared/turn-diff";
import { diffMarkers } from "./diff-colors";
import { useSelectedFile, type FileSelection } from "./selection";

type Open = (selection: FileSelection, workspaceId: string) => void;
type File = TurnDiff["files"][number];
type Theme = PluginTimelineItemProps["theme"];

const BAR_BLOCKS = 5;

export function createTurnDiffCard(open: Open) {
  return function TurnDiffCard({ agentId, item, theme }: PluginTimelineItemProps<TurnDiff>) {
    return <TurnDiffCardView agentId={agentId} data={item.data} theme={theme} open={open} />;
  };
}

function splitPath(path: string) {
  const slash = path.lastIndexOf("/");
  return slash < 0
    ? { dir: "", name: path }
    : { dir: path.slice(0, slash + 1), name: path.slice(slash + 1) };
}

/** GitHub-style diffstat blocks: the share of added and removed lines in one file. */
function barBlocks(added: number, deleted: number) {
  const total = added + deleted;
  if (total === 0) return { add: 0, remove: 0 };
  const add = Math.round((BAR_BLOCKS * added) / total);
  return { add, remove: BAR_BLOCKS - add };
}

function useStyles(theme: Theme) {
  return useMemo(() => {
    const markers = diffMarkers(theme);
    const mono = { fontFamily: "monospace", fontSize: 12 };
    return {
      iconMuted: theme.colors.foregroundMuted,
      iconAccent: theme.colors.accent,
      card: {
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 10,
        backgroundColor: theme.colors.surface1,
        overflow: "hidden" as const,
      },
      header: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 8,
        paddingHorizontal: 12,
        paddingVertical: 10,
      },
      title: { color: theme.colors.foreground, fontWeight: "600" as const },
      add: { ...mono, color: markers.add, fontWeight: "600" as const },
      remove: { ...mono, color: markers.remove, fontWeight: "600" as const },
      spacer: { flex: 1 },
      muted: { color: theme.colors.foregroundMuted, fontSize: 12 },
      section: {
        borderTopWidth: 1,
        borderColor: theme.colors.border,
        backgroundColor: theme.colors.surface0,
      },
      notice: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 6,
        paddingHorizontal: 12,
        paddingVertical: 6,
      },
      warning: { color: theme.colors.statusWarning, fontSize: 12 },
      commit: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 8,
        paddingHorizontal: 12,
        paddingVertical: 4,
      },
      sha: { ...mono, color: theme.colors.accent },
      subject: { flexShrink: 1, color: theme.colors.foreground, fontSize: 13 },
      row: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 8,
        paddingHorizontal: 12,
        paddingVertical: 6,
      },
      rowHover: { backgroundColor: theme.colors.surface2 },
      rowSelected: {
        backgroundColor: theme.colors.surface2,
        borderLeftWidth: 2,
        borderColor: theme.colors.accent,
        paddingLeft: 10,
      },
      path: { flex: 1, flexDirection: "row" as const, minWidth: 0 },
      dir: { flexShrink: 1, color: theme.colors.foregroundMuted, fontSize: 13 },
      name: {
        flexShrink: 0,
        color: theme.colors.foreground,
        fontSize: 13,
        fontWeight: "500" as const,
      },
      counts: {
        flexDirection: "row" as const,
        gap: 6,
        minWidth: 72,
        justifyContent: "flex-end" as const,
      },
      badge: {
        ...mono,
        fontSize: 11,
        color: theme.colors.foregroundMuted,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 4,
        paddingHorizontal: 4,
      },
      bar: { flexDirection: "row" as const, gap: 2 },
      block: { width: 7, height: 7, borderRadius: 1.5 },
      blockAdd: { backgroundColor: markers.add },
      blockRemove: { backgroundColor: markers.remove },
      blockNone: { backgroundColor: theme.colors.border },
      toggle: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 6,
        paddingHorizontal: 12,
        paddingVertical: 8,
        borderTopWidth: 1,
        borderColor: theme.colors.border,
      },
    };
  }, [theme]);
}

type Styles = ReturnType<typeof useStyles>;

function DiffBar({ file, styles }: { file: File; styles: Styles }) {
  const blocks =
    file.added === null ? { add: 0, remove: 0 } : barBlocks(file.added, file.deleted ?? 0);
  return (
    <View style={styles.bar}>
      {Array.from({ length: BAR_BLOCKS }, (_, index) => (
        <View
          key={index}
          style={[
            styles.block,
            index < blocks.add
              ? styles.blockAdd
              : index < blocks.add + blocks.remove
                ? styles.blockRemove
                : styles.blockNone,
          ]}
        />
      ))}
    </View>
  );
}

function FileRow({
  file,
  styles,
  selected,
  onPress,
}: {
  file: File;
  styles: Styles;
  selected: boolean;
  onPress: (() => void) | null;
}) {
  const { dir, name } = splitPath(file.path);
  const content = (hovered: boolean) => (
    <>
      <Icon
        name={imageType(file.path) ? "FileImage" : file.added === null ? "File" : "FileCode"}
        size={14}
        color={styles.iconMuted}
      />
      <View style={styles.path}>
        <Text style={styles.dir} numberOfLines={1} ellipsizeMode="head">
          {dir}
        </Text>
        <Text style={styles.name} numberOfLines={1}>
          {name}
        </Text>
      </View>
      <View style={styles.counts}>
        {file.added === null ? (
          <Text style={styles.badge}>{file.large ? "large, not saved" : "binary"}</Text>
        ) : (
          <>
            {file.added > 0 ? <Text style={styles.add}>+{file.added}</Text> : null}
            {(file.deleted ?? 0) > 0 ? <Text style={styles.remove}>-{file.deleted}</Text> : null}
          </>
        )}
      </View>
      <DiffBar file={file} styles={styles} />
      {onPress ? (
        <Icon
          name="ChevronRight"
          size={14}
          color={hovered || selected ? styles.iconAccent : styles.iconMuted}
        />
      ) : null}
    </>
  );
  if (!onPress) return <View style={styles.row}>{content(false)}</View>;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`Open the turn diff of ${file.path}`}
      onPress={onPress}
      style={(state) => {
        const hovered = (state as { hovered?: boolean }).hovered === true;
        return [
          styles.row,
          hovered || state.pressed ? styles.rowHover : null,
          selected ? styles.rowSelected : null,
        ];
      }}
    >
      {(state) => content((state as { hovered?: boolean }).hovered === true)}
    </Pressable>
  );
}

export function TurnDiffCardView({
  agentId,
  data,
  theme,
  open,
}: {
  agentId: string;
  data: TurnDiff;
  theme: Theme;
  open: Open;
}) {
  const styles = useStyles(theme);
  const selected = useSelectedFile();
  const source = data.source;
  const files = data.files;
  const hidden = data.fileCount - files.length;
  const isSelected = (path: string) =>
    selected !== null &&
    selected.agentId === agentId &&
    selected.to === source?.to &&
    selected.path === path;

  return (
    <View style={styles.card}>
      <View style={styles.header}>
        <Icon name="FileDiff" size={16} color={theme.colors.foregroundMuted} />
        <Text style={styles.title}>
          {data.fileCount} {data.fileCount === 1 ? "file" : "files"} changed
        </Text>
        {data.fileCount > 0 ? (
          <>
            <Text style={styles.add}>+{data.added}</Text>
            <Text style={styles.remove}>-{data.deleted}</Text>
          </>
        ) : null}
        <View style={styles.spacer} />
        {data.commits.length > 0 ? (
          <Text style={styles.muted}>
            {data.commits.length} {data.commits.length === 1 ? "commit" : "commits"}
          </Text>
        ) : null}
      </View>
      {data.shared || data.commits.length > 0 ? (
        <View style={styles.section}>
          {data.shared ? (
            <View style={styles.notice}>
              <Icon name="TriangleAlert" size={13} color={theme.colors.statusWarning} />
              <Text style={styles.warning}>May include changes from another agent</Text>
            </View>
          ) : null}
          {data.commits.map((commit) => (
            <View key={commit.sha} style={styles.commit}>
              <Icon name="GitCommitHorizontal" size={14} color={theme.colors.foregroundMuted} />
              <Text style={styles.sha}>{commit.sha}</Text>
              <Text style={styles.subject} numberOfLines={1}>
                {commit.subject}
              </Text>
            </View>
          ))}
        </View>
      ) : null}
      {files.length > 0 ? (
        <View style={styles.section}>
          {files.map((file) => (
            <FileRow
              key={file.path}
              file={file}
              styles={styles}
              selected={isSelected(file.path)}
              onPress={
                source && !file.large
                  ? () =>
                      open(
                        {
                          agentId,
                          root: source.root,
                          from: source.from,
                          to: source.to,
                          path: file.path,
                        },
                        source.workspaceId,
                      )
                  : null
              }
            />
          ))}
        </View>
      ) : null}
      {hidden > 0 ? (
        <View style={styles.toggle}>
          <Text style={styles.muted}>
            and {hidden} more {hidden === 1 ? "file" : "files"} not listed
          </Text>
        </View>
      ) : null}
    </View>
  );
}
