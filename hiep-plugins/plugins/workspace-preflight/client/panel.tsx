import { type PluginWorkspacePanelProps, useRpc, useWorkspace } from "@getpaseo/plugin/client";
import { ScrollView } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { Pressable, Text, View } from "react-native";
import { preflightRpc } from "../shared/preflight";

export function PreflightPanel({ workspaceId, host, theme, layout }: PluginWorkspacePanelProps) {
  const project = useWorkspace(workspaceId, (workspace) => workspace.projectDisplayName);
  const run = useRpc(preflightRpc);
  const result = useQuery({
    queryKey: ["workspace-preflight", host.id, workspaceId],
    queryFn: () => run({ workspaceId }),
    retry: false,
    staleTime: 0,
    refetchOnWindowFocus: false,
  });
  const colors = theme.colors;
  const text = { color: colors.foreground, fontSize: 14, lineHeight: 21 };
  const muted = { ...text, color: colors.foregroundMuted, fontSize: 12 };
  const checks = result.isFetching || result.isError ? [] : (result.data?.checks ?? []);
  const blockers = checks.filter((check) => check.status === "blocker").length;
  const unknowns = checks.filter((check) => check.status === "unknown").length;
  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: colors.surface0 }}
      contentContainerStyle={{ padding: layout.compact ? 12 : 20, gap: 12 }}
    >
      <Text style={{ ...text, fontSize: 20, fontWeight: "600" }}>Workspace preflight</Text>
      <Text style={muted}>{project ?? "Workspace"} · Read-only</Text>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Run workspace preflight"
        disabled={result.isFetching}
        onPress={() => void result.refetch()}
        style={{ padding: 12, borderRadius: 6, backgroundColor: colors.surface2 }}
      >
        <Text style={text}>{result.isFetching ? "Checking…" : "Run checks"}</Text>
      </Pressable>
      {result.isFetching ? (
        <Text style={muted}>Checking prerequisites…</Text>
      ) : result.isError ? (
        <Text accessibilityRole="alert" style={text}>
          Could not run preflight. Check the host connection and try Run checks again.
        </Text>
      ) : result.data ? (
        <>
          <Text style={{ ...text, fontWeight: "600" }}>
            {blockers
              ? `${blockers} blocker(s)`
              : unknowns
                ? "Readiness unknown"
                : "Measured checks passed"}
          </Text>
          <Text style={muted}>
            {checks.filter((check) => check.status === "pass").length} pass · {blockers} blocker ·{" "}
            {unknowns} unknown
          </Text>
          <Text style={muted}>
            {result.data.mode} · Checked {result.data.checkedAt}
          </Text>
          {checks.map((check) => (
            <View
              key={check.id}
              style={{
                gap: 6,
                padding: 12,
                borderRadius: 8,
                borderWidth: 1,
                borderColor: colors.border,
              }}
            >
              <Text style={{ ...text, fontWeight: "600" }}>
                {check.status.toUpperCase()} · {check.label}
              </Text>
              <Text style={text}>{check.detail}</Text>
              <Text style={muted}>{check.source}</Text>
              {check.status !== "pass" && check.repair && (
                <>
                  <Text style={muted}>Suggested command — review before running:</Text>
                  <Text selectable style={text}>
                    {check.repair}
                  </Text>
                </>
              )}
            </View>
          ))}
        </>
      ) : null}
      <Text style={muted}>
        Discovery covers bounded root metadata; explicit settings override each category. Passing
        checks do not prove task coverage. Jev evaluation is separate and optional. Suggestions
        never execute automatically.
      </Text>
    </ScrollView>
  );
}
