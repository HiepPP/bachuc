import { type PluginWorkspacePanelProps, useRpc } from "@getpaseo/plugin/client";
import { ScrollView } from "@getpaseo/plugin/client/react-native";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Pressable, Text, TextInput, View } from "react-native";
import {
  type DirectInput,
  type ModelAllowlist,
  directProfilesRpc,
  directRunRpc,
  directStatusRpc,
  isLunaModel,
} from "../shared/direct";

type DirectRecordView = {
  requestId: string;
  status: "routing" | "creating" | "created" | "failed" | "interrupted";
  agentId?: string;
  selection?: { name?: string; model?: string; thinkingOptionId?: string };
  error?: string;
};

const effortIds = ["low", "medium", "high", "xhigh", "max"] as const;
type EffortId = ModelAllowlist[number]["effortIds"][number];

type ModelGroup = {
  key: string;
  provider: string;
  model: string;
  effortIds: EffortId[];
  profileCount: number;
};

function modelKey(provider: string, model: string) {
  return JSON.stringify([provider, model]);
}

function offeredEffortIds(model: string, offered: EffortId[]) {
  const supported = effortIds.filter((effortId) => offered.includes(effortId));
  return isLunaModel(model) ? supported.filter((effortId) => effortId === "max") : supported;
}

function createRequestId() {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (character) => {
    const random = Math.floor(Math.random() * 16);
    const value = character === "x" ? random : (random & 0x3) | 0x8;
    return value.toString(16);
  });
}

function asDirectRecord(value: unknown): DirectRecordView | null {
  if (!value || typeof value !== "object") return null;
  const record = value as Record<string, unknown>;
  if (
    typeof record.requestId !== "string" ||
    !["routing", "creating", "created", "failed", "interrupted"].includes(String(record.status))
  ) {
    return null;
  }
  return value as DirectRecordView;
}

export function DirectPanel(props: PluginWorkspacePanelProps) {
  return <DirectPanelState key={`${props.host.id}:${props.workspaceId}`} {...props} />;
}

function DirectPanelState({
  workspaceId,
  host,
  theme,
  layout,
  navigation,
}: PluginWorkspacePanelProps) {
  const getProfiles = useRpc(directProfilesRpc);
  const runDirect = useRpc(directRunRpc);
  const getStatus = useRpc(directStatusRpc);
  const [prompt, setPrompt] = useState("");
  const [selectedProfileIds, setSelectedProfileIds] = useState<string[]>([]);
  const [selectedEfforts, setSelectedEfforts] = useState<Record<string, EffortId[]>>({});
  const [request, setRequest] = useState<DirectInput | null>(null);

  const profiles = useQuery({
    queryKey: ["jev-direct-profiles", host.id, workspaceId],
    queryFn: () => getProfiles({ workspaceId }),
    retry: false,
    staleTime: 0,
    refetchOnWindowFocus: false,
  });
  const run = useMutation({
    mutationFn: (input: DirectInput) => runDirect(input),
    retry: false,
  });
  const status = useMutation({
    mutationFn: (input: { workspaceId: string; requestId: string }) => getStatus(input),
    retry: false,
  });

  const colors = theme.colors;
  const text = { color: colors.foreground, fontSize: 14, lineHeight: 21 };
  const muted = { ...text, color: colors.foregroundMuted, fontSize: 12 };
  const locked = request !== null;
  const selectedModelGroups = (() => {
    const groups = new Map<string, ModelGroup>();
    for (const profile of profiles.data?.profiles ?? []) {
      if (!selectedProfileIds.includes(profile.id)) continue;
      const key = modelKey(profile.provider, profile.model);
      const offered = offeredEffortIds(profile.model, profile.effortIds);
      const existing = groups.get(key);
      if (existing) {
        existing.effortIds = effortIds.filter(
          (effortId) => existing.effortIds.includes(effortId) || offered.includes(effortId),
        );
        existing.profileCount += 1;
      } else {
        groups.set(key, {
          key,
          provider: profile.provider,
          model: profile.model,
          effortIds: offered,
          profileCount: 1,
        });
      }
    }
    return [...groups.values()];
  })();
  const selectedAllowedModels = selectedModelGroups.map((group) => ({
    provider: group.provider,
    model: group.model,
    effortIds: (selectedEfforts[group.key] ?? []).filter((effortId) =>
      group.effortIds.includes(effortId),
    ),
  }));
  const hasEffortForEveryModel =
    selectedAllowedModels.length > 0 &&
    selectedAllowedModels.every((model) => model.effortIds.length > 0);
  const matchingRecord = request
    ? status.data?.records
        .map(asDirectRecord)
        .find((record): record is DirectRecordView => record?.requestId === request.requestId)
    : undefined;
  const result = run.data;
  const agentId =
    result?.agentId ?? (matchingRecord?.status === "created" ? matchingRecord.agentId : undefined);
  const selection =
    result?.selection ??
    (matchingRecord?.status === "created" ? matchingRecord.selection : undefined);
  const mayReset = run.isSuccess || ["created", "failed"].includes(matchingRecord?.status ?? "");
  const mayRetrySameRequest =
    run.isError && status.isSuccess && !matchingRecord && status.data.records.length === 0;
  const canRoute =
    !run.isPending &&
    !status.isPending &&
    ((request === null &&
      prompt.trim().length >= 8 &&
      selectedProfileIds.length > 0 &&
      hasEffortForEveryModel) ||
      mayRetrySameRequest);

  function route() {
    const nextRequest =
      request ??
      ({
        workspaceId,
        requestId: createRequestId(),
        prompt,
        allowedProfileIds: [...selectedProfileIds],
        allowedModels: selectedAllowedModels,
        shareWithJev: true,
      } satisfies DirectInput);
    if (!request) setRequest(nextRequest);
    status.reset();
    run.mutate(nextRequest);
  }

  function reset() {
    setPrompt("");
    setSelectedProfileIds([]);
    setSelectedEfforts({});
    setRequest(null);
    run.reset();
    status.reset();
  }

  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: colors.surface0 }}
      contentContainerStyle={{ padding: layout.compact ? 12 : 20, gap: 14 }}
    >
      <Text style={{ ...text, fontSize: 20, fontWeight: "600" }}>Direct Jev routing</Text>
      <Text style={muted}>
        Choose profiles, then explicitly allow effort levels for every selected model. Jev can only
        choose an allowed profile, model, and effort combination. Luna offers max effort only. The
        selected profile's mode and feature values stay unchanged. The current agent is not
        involved.
      </Text>

      <Text style={{ ...text, fontWeight: "600" }}>Prompt</Text>
      <TextInput
        accessibilityLabel="Direct routing prompt"
        editable={!locked}
        maxLength={12000}
        multiline
        onChangeText={setPrompt}
        placeholder="Describe the task for the new agent"
        placeholderTextColor={colors.foregroundMuted}
        style={{
          minHeight: 120,
          padding: 12,
          borderRadius: 8,
          borderWidth: 1,
          borderColor: colors.border,
          backgroundColor: colors.surface1,
          color: colors.foreground,
          textAlignVertical: "top",
        }}
        value={prompt}
      />

      <Text style={{ ...text, fontWeight: "600" }}>Allowed profiles</Text>
      <Text style={muted}>Select at least one profile. Jev only chooses from this list.</Text>
      {profiles.isPending ? <Text style={muted}>Loading profiles…</Text> : null}
      {profiles.isError ? (
        <View style={{ gap: 8 }}>
          <Text accessibilityRole="alert" style={{ ...text, color: colors.statusDanger }}>
            Could not load profiles.
          </Text>
          <Pressable
            accessibilityRole="button"
            disabled={profiles.isFetching}
            onPress={() => void profiles.refetch()}
            style={{ padding: 10, borderRadius: 6, backgroundColor: colors.surface2 }}
          >
            <Text style={text}>{profiles.isFetching ? "Retrying…" : "Retry profiles"}</Text>
          </Pressable>
        </View>
      ) : null}
      {profiles.data?.profiles.length === 0 ? (
        <Text style={muted}>No direct-routing profiles are available for this workspace.</Text>
      ) : null}
      {profiles.data?.profiles.map((profile) => {
        const selected = selectedProfileIds.includes(profile.id);
        const profileEffortIds = offeredEffortIds(profile.model, profile.effortIds);
        const profileDisabled = profileEffortIds.length === 0;
        const selectionLimitReached = !selected && selectedProfileIds.length >= 8;
        return (
          <Pressable
            accessibilityRole="checkbox"
            accessibilityState={{
              checked: selected,
              disabled: locked || selectionLimitReached || profileDisabled,
            }}
            disabled={locked || selectionLimitReached || profileDisabled}
            key={profile.id}
            onPress={() => {
              const nextProfileIds = selected
                ? selectedProfileIds.filter((id) => id !== profile.id)
                : [...selectedProfileIds, profile.id];
              setSelectedProfileIds(nextProfileIds);
              const nextModelKeys = new Set(
                (profiles.data?.profiles ?? [])
                  .filter((candidate) => nextProfileIds.includes(candidate.id))
                  .map((candidate) => modelKey(candidate.provider, candidate.model)),
              );
              setSelectedEfforts((current) =>
                Object.fromEntries(
                  Object.entries(current).filter(([key]) => nextModelKeys.has(key)),
                ),
              );
            }}
            style={{
              gap: 4,
              padding: 12,
              borderRadius: 8,
              borderWidth: 1,
              borderColor: selected ? colors.accent : colors.border,
              backgroundColor: selected ? colors.surface2 : colors.surface1,
              opacity: locked || selectionLimitReached || profileDisabled ? 0.65 : 1,
            }}
          >
            <Text style={{ ...text, fontWeight: "600" }}>
              {selected ? "Selected · " : ""}
              {profile.name}
            </Text>
            <Text style={muted}>
              {profile.provider} · {profile.model} · mode {profile.modeId ?? "default"}
            </Text>
            {profileDisabled ? (
              <Text style={{ ...muted, color: colors.statusDanger }}>
                No effort levels are currently available for this profile.
              </Text>
            ) : null}
            {profile.notes ? <Text style={text}>{profile.notes}</Text> : null}
          </Pressable>
        );
      })}

      {selectedModelGroups.length > 0 ? (
        <View style={{ gap: 10 }}>
          <Text style={{ ...text, fontWeight: "600" }}>Allowed model efforts</Text>
          <Text style={muted}>
            Select at least one offered effort for each model. Only checked efforts are sent to Jev.
          </Text>
          {selectedModelGroups.map((group) => (
            <View
              key={group.key}
              style={{
                gap: 8,
                padding: 12,
                borderRadius: 8,
                borderWidth: 1,
                borderColor: colors.border,
                backgroundColor: colors.surface1,
              }}
            >
              <Text style={{ ...text, fontWeight: "600" }}>
                {group.provider} · {group.model}
              </Text>
              {group.profileCount > 1 ? (
                <Text style={muted}>{group.profileCount} selected profiles use this model.</Text>
              ) : null}
              <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 8 }}>
                {group.effortIds.map((effortId) => {
                  const selected = selectedEfforts[group.key]?.includes(effortId) ?? false;
                  return (
                    <Pressable
                      accessibilityRole="checkbox"
                      accessibilityState={{ checked: selected, disabled: locked }}
                      disabled={locked}
                      key={effortId}
                      onPress={() =>
                        setSelectedEfforts((current) => {
                          const currentEfforts = current[group.key] ?? [];
                          const nextEfforts = selected
                            ? currentEfforts.filter((candidate) => candidate !== effortId)
                            : effortIds.filter(
                                (candidate) =>
                                  currentEfforts.includes(candidate) || candidate === effortId,
                              );
                          return { ...current, [group.key]: nextEfforts };
                        })
                      }
                      style={{
                        paddingHorizontal: 12,
                        paddingVertical: 8,
                        borderRadius: 6,
                        borderWidth: 1,
                        borderColor: selected ? colors.accent : colors.border,
                        backgroundColor: selected ? colors.surface2 : colors.surface0,
                        opacity: locked ? 0.65 : 1,
                      }}
                    >
                      <Text style={text}>
                        {selected ? "Selected · " : ""}
                        {effortId}
                      </Text>
                    </Pressable>
                  );
                })}
              </View>
            </View>
          ))}
        </View>
      ) : null}

      <Pressable
        accessibilityRole="button"
        accessibilityLabel={request ? "Retry the same direct routing request" : "Route and run"}
        disabled={!canRoute}
        onPress={route}
        style={{
          padding: 12,
          borderRadius: 6,
          backgroundColor: colors.accent,
          opacity: canRoute ? 1 : 0.55,
        }}
      >
        <Text style={{ color: colors.accentForeground, fontWeight: "600", textAlign: "center" }}>
          {run.isPending
            ? "Routing…"
            : mayRetrySameRequest
              ? "Retry same request"
              : "Route and run"}
        </Text>
      </Pressable>

      {request ? (
        <Text selectable style={muted}>
          Request {request.requestId}
        </Text>
      ) : null}
      {run.isError ? (
        <View style={{ gap: 8 }}>
          <Text accessibilityRole="alert" style={{ ...text, color: colors.statusDanger }}>
            The result is uncertain. Check the saved request before retrying or starting another
            task.
          </Text>
          <Pressable
            accessibilityRole="button"
            disabled={status.isPending}
            onPress={() => request && status.mutate({ workspaceId, requestId: request.requestId })}
            style={{ padding: 10, borderRadius: 6, backgroundColor: colors.surface2 }}
          >
            <Text style={text}>{status.isPending ? "Checking…" : "Check request status"}</Text>
          </Pressable>
        </View>
      ) : null}
      {status.isError ? (
        <Text accessibilityRole="alert" style={{ ...text, color: colors.statusDanger }}>
          Could not check the saved request. Try checking again with the same request ID.
        </Text>
      ) : null}
      {status.isSuccess && !matchingRecord ? (
        <Text style={muted}>
          No saved record was found. A retry will reuse the same request ID.
        </Text>
      ) : null}
      {matchingRecord ? (
        <View
          style={{
            gap: 6,
            padding: 12,
            borderRadius: 8,
            borderWidth: 1,
            borderColor: colors.border,
          }}
        >
          <Text style={{ ...text, fontWeight: "600" }}>
            Saved request · {matchingRecord.status}
          </Text>
          {matchingRecord.error ? <Text style={text}>{matchingRecord.error}</Text> : null}
          {matchingRecord.status === "interrupted" ? (
            <Text style={muted}>
              This outcome is uncertain. Keep this request ID and inspect the saved agent labels.
            </Text>
          ) : null}
        </View>
      ) : null}
      {selection ? (
        <View
          style={{
            gap: 6,
            padding: 12,
            borderRadius: 8,
            borderWidth: 1,
            borderColor: colors.border,
          }}
        >
          <Text style={{ ...text, fontWeight: "600" }}>Agent created</Text>
          <Text style={text}>
            {selection.name ? `${selection.name} · ` : ""}
            {selection.model ?? "Unknown model"} · effort {selection.thinkingOptionId ?? "unknown"}
          </Text>
          {agentId && navigation ? (
            <Pressable
              accessibilityRole="button"
              onPress={() => navigation.openAgent({ agentId })}
              style={{ padding: 10, borderRadius: 6, backgroundColor: colors.surface2 }}
            >
              <Text style={text}>Open agent</Text>
            </Pressable>
          ) : null}
        </View>
      ) : null}
      {mayReset ? (
        <Pressable
          accessibilityRole="button"
          onPress={reset}
          style={{ padding: 10, borderRadius: 6, backgroundColor: colors.surface2 }}
        >
          <Text style={text}>Reset for a new task</Text>
        </Pressable>
      ) : null}
    </ScrollView>
  );
}
