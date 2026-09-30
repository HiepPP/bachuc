import { useRpc, type PluginAgentPanelProps } from "@getpaseo/plugin/client";
import type { PluginTheme } from "@getpaseo/plugin";
import { Icon, ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Image, Platform, Pressable, Text, View, type TextStyle } from "react-native";
import {
  fileDiffRpc,
  fileImageRpc,
  turnHistoryRpc,
  imageType,
  parseUnifiedDiff,
  type DiffRow,
} from "../shared/turn-diff";
import { ADD_BACKGROUND, diffMarkers, REMOVE_BACKGROUND } from "./diff-colors";
import { selectFile, useSelectedFile, type FileSelection } from "./selection";
import { TurnDiffCardView } from "./turn-diff-card";

const MAX_ROWS = 2_000;
const FONT_SIZE = 12;
const LINE_HEIGHT = 20;
// Web text wraps by default; code lines scroll sideways instead, as in Paseo.
const NO_WRAP = (Platform.OS === "web" ? { whiteSpace: "pre" } : {}) as TextStyle;
// Wrapped lines keep indentation and break long tokens such as URLs or paths.
const WRAP = (
  Platform.OS === "web" ? { whiteSpace: "pre-wrap", overflowWrap: "anywhere" } : {}
) as TextStyle;
// Long changed lines are the common case, so wrapping starts on; the choice holds for the session.
let wrapPreference = true;

/** Paseo's gutter formula: at least 2 digits, each about 0.62 of the font size, plus padding. */
function gutterWidth(maxLine: number) {
  return Math.max(2, String(maxLine).length) * Math.ceil(0.62 * FONT_SIZE) + 12;
}

function useStyles(theme: PluginTheme, gutter: number) {
  return useMemo(() => {
    const markers = diffMarkers(theme);
    const mono = { fontFamily: "monospace", fontSize: FONT_SIZE, lineHeight: LINE_HEIGHT };
    return {
      screen: { flex: 1, backgroundColor: theme.colors.surface0 },
      header: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 8,
        paddingHorizontal: 12,
        paddingVertical: 8,
        borderBottomWidth: 1,
        borderColor: theme.colors.border,
        backgroundColor: theme.colors.surface1,
      },
      path: { ...mono, flexShrink: 1, color: theme.colors.foreground, fontWeight: "600" as const },
      badge: {
        fontSize: 11,
        color: theme.colors.foregroundMuted,
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 4,
        paddingHorizontal: 6,
      },
      addCount: { ...mono, color: markers.add },
      removeCount: { ...mono, color: markers.remove },
      message: { padding: 12, color: theme.colors.foregroundMuted },
      error: { padding: 12, color: theme.colors.statusDanger },
      lines: { alignSelf: "flex-start" as const, minWidth: "100%" as const, paddingVertical: 4 },
      row: { flexDirection: "row" as const, minWidth: "100%" as const },
      add: { backgroundColor: ADD_BACKGROUND },
      remove: { backgroundColor: REMOVE_BACKGROUND },
      hunk: { backgroundColor: theme.colors.surface1 },
      gutter: {
        ...mono,
        width: gutter,
        paddingRight: 8,
        textAlign: "right" as const,
        color: theme.colors.foregroundMuted,
      },
      hunkGutter: { width: gutter * 2 + 16 },
      marker: { ...mono, width: 16, textAlign: "center" as const },
      markerAdd: { color: markers.add },
      markerRemove: { color: markers.remove },
      code: { ...mono, paddingRight: 16, color: theme.colors.foreground, ...NO_WRAP },
      codeWrap: {
        ...mono,
        flex: 1,
        flexShrink: 1,
        paddingRight: 12,
        color: theme.colors.foreground,
        ...WRAP,
      },
      linesWrap: { width: "100%" as const, paddingVertical: 4 },
      spacer: { flex: 1 },
      wrapToggle: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 4,
        paddingHorizontal: 8,
        paddingVertical: 3,
        borderRadius: 6,
        borderWidth: 1,
        borderColor: theme.colors.border,
      },
      wrapToggleOn: { backgroundColor: theme.colors.surface2 },
      wrapText: { fontSize: 12, color: theme.colors.foregroundMuted },
      wrapTextOn: { color: theme.colors.foreground },
      muted: { color: theme.colors.foregroundMuted },
      back: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        gap: 2,
        paddingRight: 4,
      },
      backText: { color: theme.colors.accent, fontSize: 12, fontWeight: "500" as const },
      historyContent: { padding: 12, gap: 14 },
      historyTitle: { color: theme.colors.foreground, fontWeight: "600" as const },
      turn: { gap: 6 },
      turnTime: { color: theme.colors.foregroundMuted, fontSize: 12 },
      accent: theme.colors.accent,
      images: { flexDirection: "row" as const, flexWrap: "wrap" as const, gap: 12, padding: 12 },
      imageSide: { flexGrow: 1, flexBasis: 240, gap: 6 },
      imageLabel: { fontSize: 11, color: theme.colors.foregroundMuted },
      imageFrame: {
        borderWidth: 1,
        borderColor: theme.colors.border,
        borderRadius: 6,
        backgroundColor: theme.colors.surface1,
        padding: 8,
      },
      image: { width: "100%" as const, height: 360 },
    };
  }, [theme, gutter]);
}

type Styles = ReturnType<typeof useStyles>;

function formatTime(iso: string) {
  return new Date(iso).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Every recorded turn of this agent, read from the journal, so it survives a daemon restart. */
function TurnHistory({
  agentId,
  host,
  theme,
  styles,
}: {
  agentId: string;
  host: string;
  theme: PluginAgentPanelProps["theme"];
  styles: Styles;
}) {
  const readHistory = useRpc(turnHistoryRpc);
  const history = useQuery({
    queryKey: ["thread-branch-turn-history", host, agentId],
    queryFn: () => readHistory({ agentId }),
    refetchInterval: 15_000,
    retry: false,
  });
  const turns = history.data?.turns ?? [];
  return (
    <ScrollView style={styles.screen} contentContainerStyle={styles.historyContent}>
      <Text style={styles.historyTitle}>Changes by turn</Text>
      {history.isPending ? <Text style={styles.muted}>Loading…</Text> : null}
      {history.error ? <Text style={styles.error}>{history.error.message}</Text> : null}
      {history.data && turns.length === 0 ? (
        <Text style={styles.muted}>
          No turn changes recorded for this agent in the last 30 days.
        </Text>
      ) : null}
      {turns.map((turn) => (
        <View key={turn.key} style={styles.turn}>
          <Text style={styles.turnTime}>{formatTime(turn.endedAt)}</Text>
          <TurnDiffCardView agentId={agentId} data={turn.diff} theme={theme} open={selectFile} />
        </View>
      ))}
    </ScrollView>
  );
}

function ImagePreview({
  file,
  host,
  styles,
}: {
  file: FileSelection;
  host: string;
  styles: Styles;
}) {
  const readImage = useRpc(fileImageRpc);
  const image = useQuery({
    queryKey: ["thread-branch-file-image", host, file.root, file.from, file.to, file.path],
    queryFn: () => readImage({ root: file.root, from: file.from, to: file.to, path: file.path }),
    staleTime: Infinity,
    retry: false,
  });
  if (image.isPending) return <Text style={styles.message}>Loading image…</Text>;
  if (image.error) return <Text style={styles.error}>{image.error.message}</Text>;
  const sides = [
    { label: "Before", uri: image.data.before },
    { label: "After", uri: image.data.after },
  ].filter((side): side is { label: string; uri: string } => side.uri !== null);
  return (
    <View style={styles.images}>
      {sides.map((side) => (
        <View key={side.label} style={styles.imageSide}>
          {sides.length > 1 ? <Text style={styles.imageLabel}>{side.label}</Text> : null}
          <View style={styles.imageFrame}>
            <Image
              accessibilityLabel={`${side.label}: ${file.path}`}
              source={{ uri: side.uri }}
              resizeMode="contain"
              style={styles.image}
            />
          </View>
        </View>
      ))}
      {image.data.tooLarge ? (
        <Text style={styles.message}>An image over 3 MB is not previewed.</Text>
      ) : null}
    </View>
  );
}

function DiffLine({ row, styles, wrap }: { row: DiffRow; styles: Styles; wrap: boolean }) {
  const code = wrap ? styles.codeWrap : styles.code;
  if (row.type === "hunk" || row.type === "note") {
    return (
      <View style={[styles.row, row.type === "hunk" ? styles.hunk : null]}>
        <View style={styles.hunkGutter} />
        <Text style={[code, styles.muted]}>{row.text}</Text>
      </View>
    );
  }
  const tone = row.type === "add" ? styles.add : row.type === "remove" ? styles.remove : null;
  const marker = row.type === "add" ? "+" : row.type === "remove" ? "-" : " ";
  return (
    <View style={[styles.row, tone]}>
      <Text style={styles.gutter}>{row.oldLine ?? ""}</Text>
      <Text style={styles.gutter}>{row.newLine ?? ""}</Text>
      <Text
        style={[
          styles.marker,
          row.type === "add"
            ? styles.markerAdd
            : row.type === "remove"
              ? styles.markerRemove
              : null,
        ]}
      >
        {marker}
      </Text>
      <Text style={code}>{row.text || " "}</Text>
    </View>
  );
}

export function TurnDiffPanel({ agentId, host, theme }: PluginAgentPanelProps) {
  const current = useSelectedFile();
  const file = current?.agentId === agentId ? current : null;
  const readDiff = useRpc(fileDiffRpc);
  // Snapshot trees never change, so a diff is cached for good once read.
  const diff = useQuery({
    queryKey: ["thread-branch-file-diff", host.id, file?.root, file?.from, file?.to, file?.path],
    queryFn: () =>
      file
        ? readDiff({ root: file.root, from: file.from, to: file.to, path: file.path })
        : Promise.reject(new Error("No file selected.")),
    enabled: file !== null,
    staleTime: Infinity,
    retry: false,
  });
  const parsed = useMemo(() => (diff.data ? parseUnifiedDiff(diff.data.diff) : null), [diff.data]);
  const maxLine = parsed
    ? parsed.rows.reduce(
        (max, row) => ("oldLine" in row ? Math.max(max, row.oldLine ?? 0, row.newLine ?? 0) : max),
        0,
      )
    : 0;
  const styles = useStyles(theme, gutterWidth(maxLine));
  const [wrap, setWrap] = useState(wrapPreference);

  if (!file) {
    return <TurnHistory agentId={agentId} host={host.id} theme={theme} styles={styles} />;
  }

  const badge = parsed?.binary
    ? "Binary"
    : parsed?.status === "added"
      ? "New file"
      : parsed?.status === "deleted"
        ? "Deleted"
        : null;
  const rows = parsed?.rows ?? [];
  const isImage = imageType(file.path) !== null;
  return (
    <View style={styles.screen}>
      <View style={styles.header}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Show all turns"
          onPress={() => selectFile(null)}
          style={styles.back}
        >
          <Icon name="ChevronLeft" size={14} color={styles.accent} />
          <Text style={styles.backText}>All turns</Text>
        </Pressable>
        <Text style={styles.path} numberOfLines={1}>
          {file.path}
        </Text>
        {badge ? <Text style={styles.badge}>{badge}</Text> : null}
        {parsed && !parsed.binary ? (
          <>
            <Text style={styles.addCount}>+{parsed.added}</Text>
            <Text style={styles.removeCount}>-{parsed.removed}</Text>
          </>
        ) : null}
        <View style={styles.spacer} />
        {rows.length > 0 ? (
          <Pressable
            accessibilityRole="switch"
            accessibilityState={{ checked: wrap }}
            accessibilityLabel="Wrap long lines"
            onPress={() => {
              wrapPreference = !wrap;
              setWrap(!wrap);
            }}
            style={[styles.wrapToggle, wrap ? styles.wrapToggleOn : null]}
          >
            <Icon name="WrapText" size={13} color={wrap ? styles.accent : styles.muted.color} />
            <Text style={[styles.wrapText, wrap ? styles.wrapTextOn : null]}>Wrap</Text>
          </Pressable>
        ) : null}
      </View>
      {diff.isPending ? <Text style={styles.message}>Loading…</Text> : null}
      {diff.error ? <Text style={styles.error}>{diff.error.message}</Text> : null}
      {isImage ? (
        <ScrollView style={rows.length > 0 ? null : styles.screen}>
          <ImagePreview file={file} host={host.id} styles={styles} />
        </ScrollView>
      ) : parsed?.binary ? (
        <Text style={styles.message}>Binary file. No line diff to show.</Text>
      ) : null}
      {parsed && !parsed.binary && rows.length === 0 ? (
        <Text style={styles.message}>No changes to show for this file.</Text>
      ) : null}
      {rows.length > 0 ? (
        <ScrollView style={styles.screen}>
          {wrap ? (
            <View style={styles.linesWrap}>
              {rows.slice(0, MAX_ROWS).map((row, index) => (
                <DiffLine key={index} row={row} styles={styles} wrap />
              ))}
            </View>
          ) : (
            <ScrollView horizontal>
              <View style={styles.lines}>
                {rows.slice(0, MAX_ROWS).map((row, index) => (
                  <DiffLine key={index} row={row} styles={styles} wrap={false} />
                ))}
              </View>
            </ScrollView>
          )}
          {rows.length > MAX_ROWS || diff.data?.truncated ? (
            <Text style={styles.message}>The diff is too long; only the start is shown.</Text>
          ) : null}
        </ScrollView>
      ) : null}
    </View>
  );
}
