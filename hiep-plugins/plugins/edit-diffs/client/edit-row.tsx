import type { PluginTheme } from "@getpaseo/plugin";
import {
  useAgent,
  useRpc,
  useSettings,
  type PluginTimelineItemProps,
} from "@getpaseo/plugin/client";
import { Icon, ScrollView } from "@getpaseo/plugin/client/react-native";
import { SyntaxToken, type SyntaxTokenData } from "@getpaseo/plugin/client/ui";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { Platform, Pressable, Text, View, type TextStyle } from "react-native";
import { locateRpc, type EditRowData } from "../shared/contracts";
import {
  applyLocations,
  buildHunks,
  countChanges,
  needsLocating,
  numberRows,
  sideText,
  type Hunk,
  type NumberedRow,
} from "../shared/diff";
import { displaySettings } from "../shared/settings";
import { highlightHunks } from "./highlight";

const MONO = Platform.select({
  ios: "ui-monospace",
  web: "SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace",
  default: "monospace",
});
// Keeps long lines on one row inside the horizontal scroll; React Native has no such style type.
const NO_WRAP = (Platform.OS === "web" ? { whiteSpace: "pre" } : {}) as TextStyle;
const ADD_BG = "rgba(46, 160, 67, 0.15)";
const REMOVE_BG = "rgba(248, 81, 73, 0.1)";
const MAX_DIFF_HEIGHT = 480;
const SETTLE_MS = 300;

// Streaming tool input changes many times a second; locate only once it stops changing.
function useSettled<T>(value: T): T {
  const [settled, setSettled] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), SETTLE_MS);
    return () => clearTimeout(timer);
  }, [value]);
  return settled;
}

function displayPath(filePath: string, cwd: string | null): string {
  if (!cwd) return filePath;
  const root = cwd.replace(/\/$/, "");
  // macOS reports /tmp and /var paths through their /private real path.
  for (const prefix of [`${root}/`, `/private${root}/`]) {
    if (filePath.startsWith(prefix)) return filePath.slice(prefix.length);
  }
  return filePath;
}

function headerLabel({ action, status }: EditRowData): string {
  const verb = action === "write" ? "Write" : "Edit";
  if (status === "running") return action === "write" ? "Writing" : "Editing";
  if (status === "failed") return `${verb} failed`;
  if (status === "canceled") return `${verb} canceled`;
  return verb;
}

function useLocatedHunks(data: EditRowData, cwd: string | null, enabled: boolean): Hunk[] {
  const hunks = useMemo(() => buildHunks(data), [data]);
  const request = useMemo(
    () => ({
      filePath: data.filePath,
      cwd,
      hunks: hunks.map((hunk) => ({
        oldText: sideText(hunk, "old"),
        newText: sideText(hunk, "new"),
      })),
    }),
    [cwd, data.filePath, hunks],
  );
  const settled = useSettled(JSON.stringify(request));
  const locate = useRpc(locateRpc);
  const query = useQuery({
    queryKey: ["edit-diffs", "locate", data.status, settled],
    queryFn: () => locate(JSON.parse(settled)),
    enabled: enabled && needsLocating(hunks) && settled === JSON.stringify(request),
    staleTime: Number.POSITIVE_INFINITY,
  });
  return useMemo(
    () => (query.data ? applyLocations(hunks, query.data.located) : hunks),
    [hunks, query.data],
  );
}

function DiffRow({
  row,
  tokens,
  gutterWidth,
  theme,
}: {
  row: NumberedRow;
  tokens: SyntaxTokenData[] | null;
  gutterWidth: number;
  theme: PluginTheme;
}) {
  const code: TextStyle = {
    fontFamily: MONO,
    fontSize: 12,
    lineHeight: 18,
    color: row.kind === "context" ? theme.colors.foregroundMuted : theme.colors.foreground,
    ...NO_WRAP,
  };
  const prefix = row.kind === "add" ? "+" : row.kind === "remove" ? "-" : " ";
  const prefixColor =
    row.kind === "add"
      ? theme.colors.statusSuccess
      : row.kind === "remove"
        ? theme.colors.statusDanger
        : theme.colors.foregroundMuted;
  return (
    <View
      style={{
        flexDirection: "row",
        paddingRight: 12,
        backgroundColor:
          row.kind === "add" ? ADD_BG : row.kind === "remove" ? REMOVE_BG : "transparent",
      }}
    >
      <Text
        selectable={false}
        style={{ ...code, color: theme.colors.foregroundMuted, paddingLeft: 8, paddingRight: 8 }}
      >
        {(row.line === null ? "" : String(row.line)).padStart(gutterWidth, " ")}
      </Text>
      <Text style={code}>
        <Text style={{ color: prefixColor }}>{`${prefix} `}</Text>
        {tokens
          ? tokens.map((token, index) => <SyntaxToken key={index} token={token} />)
          : row.text}
      </Text>
    </View>
  );
}

function DiffBody({
  hunks,
  filePath,
  theme,
}: {
  hunks: Hunk[];
  filePath: string;
  theme: PluginTheme;
}) {
  const numbered = useMemo(() => hunks.map(numberRows), [hunks]);
  const tokens = useMemo(() => highlightHunks(hunks, filePath), [filePath, hunks]);
  const gutterWidth = Math.max(
    1,
    ...numbered.flat().map((row) => (row.line === null ? 0 : String(row.line).length)),
  );
  // A percentage min-width resolves against the scroll content, so measure the viewport instead.
  const [viewportWidth, setViewportWidth] = useState(0);
  return (
    <ScrollView style={{ maxHeight: MAX_DIFF_HEIGHT }} nestedScrollEnabled>
      <ScrollView
        horizontal
        nestedScrollEnabled
        onLayout={(event) => setViewportWidth(event.nativeEvent.layout.width)}
      >
        <View style={{ paddingVertical: 6, minWidth: viewportWidth }}>
          {numbered.map((rows, hunkIndex) => (
            <View key={hunkIndex}>
              {hunkIndex > 0 ? (
                <Text
                  style={{
                    fontFamily: MONO,
                    fontSize: 12,
                    lineHeight: 18,
                    paddingLeft: 8,
                    color: theme.colors.foregroundMuted,
                  }}
                >
                  {"⋮".padStart(gutterWidth, " ")}
                </Text>
              ) : null}
              {rows.map((row, rowIndex) => (
                <DiffRow
                  key={rowIndex}
                  row={row}
                  tokens={tokens[hunkIndex]?.[rowIndex] ?? null}
                  gutterWidth={gutterWidth}
                  theme={theme}
                />
              ))}
            </View>
          ))}
        </View>
      </ScrollView>
    </ScrollView>
  );
}

export function EditDiffRow({ agentId, item, theme }: PluginTimelineItemProps<EditRowData>) {
  const data = item.data;
  const settings = useSettings(displaySettings);
  const autoExpand = settings.status === "ready" ? settings.values.autoExpand : false;
  const [toggled, setToggled] = useState<boolean | null>(null);
  const expanded = toggled ?? autoExpand;
  const cwd = useAgent(agentId, (agent) => agent.cwd);
  const hunks = useLocatedHunks(data, cwd, expanded);
  const { added, removed } = useMemo(() => countChanges(hunks), [hunks]);
  const failed = data.status === "failed";
  const muted = theme.colors.foregroundMuted;

  return (
    <View style={{ marginHorizontal: -13, marginBottom: 4 }}>
      <Pressable
        accessibilityRole="button"
        accessibilityState={{ expanded }}
        accessibilityLabel={`${headerLabel(data)} ${data.filePath}`}
        onPress={() => setToggled(!expanded)}
        style={{
          flexDirection: "row",
          alignItems: "center",
          gap: 8,
          paddingHorizontal: 8,
          paddingVertical: 4,
          borderRadius: 8,
        }}
      >
        <View style={{ width: 22, alignItems: "center" }}>
          <Icon name="Pencil" size={14} color={failed ? theme.colors.statusDanger : muted} />
        </View>
        <Text style={{ color: failed ? theme.colors.statusDanger : muted, fontSize: 14 }}>
          {headerLabel(data)}
        </Text>
        <Text
          numberOfLines={1}
          style={{ flexShrink: 1, minWidth: 0, color: theme.colors.foreground, fontSize: 14 }}
        >
          {displayPath(data.filePath, cwd)}
        </Text>
        {added > 0 ? (
          <Text style={{ color: theme.colors.statusSuccess, fontSize: 13 }}>{`+${added}`}</Text>
        ) : null}
        {removed > 0 ? (
          <Text style={{ color: theme.colors.statusDanger, fontSize: 13 }}>{`−${removed}`}</Text>
        ) : null}
        <Icon name={expanded ? "ChevronDown" : "ChevronRight"} size={14} color={muted} />
      </Pressable>
      {expanded ? (
        <View
          style={{
            marginTop: 4,
            marginHorizontal: 8,
            borderWidth: 1,
            borderColor: theme.colors.border,
            borderRadius: 8,
            overflow: "hidden",
            backgroundColor: theme.colors.surface1,
          }}
        >
          {data.error ? (
            <Text style={{ color: theme.colors.statusDanger, fontSize: 13, padding: 8 }} selectable>
              {data.error}
            </Text>
          ) : null}
          {hunks.length > 0 ? (
            <DiffBody hunks={hunks} filePath={data.filePath} theme={theme} />
          ) : (
            <Text style={{ color: muted, fontSize: 13, padding: 8 }}>No changes to show.</Text>
          )}
        </View>
      ) : null}
    </View>
  );
}
