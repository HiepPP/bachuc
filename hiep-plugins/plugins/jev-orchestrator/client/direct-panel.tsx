import { type PluginWorkspacePanelProps, useRpc } from "@getpaseo/plugin/client";
import { ScrollView } from "@getpaseo/plugin/client/react-native";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useCallback, useMemo, useState } from "react";
import { Pressable, Text, TextInput, View } from "react-native";
import {
  type DirectInput,
  type ModelAllowlist,
  directProfilesRpc,
  directRunRpc,
  directStatusRpc,
  isLunaModel,
} from "../shared/direct";

interface DirectRecordView {
  requestId: string;
  status: "routing" | "creating" | "created" | "failed" | "interrupted";
  agentId?: string;
  selection?: { name?: string; model?: string; thinkingOptionId?: string };
  error?: string;
}

const effortIds = ["low", "medium", "high", "xhigh", "max"] as const;
type EffortId = ModelAllowlist[number]["effortIds"][number];

interface ModelGroup {
  key: string;
  provider: string;
  model: string;
  effortIds: EffortId[];
  profileCount: number;
}

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

type Colors = PluginWorkspacePanelProps["theme"]["colors"];
type Navigation = PluginWorkspacePanelProps["navigation"];
type Styles = ReturnType<typeof createStyles>;

interface ProfileView {
  id: string;
  name: string;
  provider: string;
  model: string;
  modeId?: string;
  notes?: string;
  effortIds: EffortId[];
}

interface SelectionView {
  name?: string;
  model?: string;
  thinkingOptionId?: string;
}

function createStyles(colors: Colors, compact: boolean) {
  const text = { color: colors.foreground, fontSize: 14, lineHeight: 21 };
  const muted = { ...text, color: colors.foregroundMuted, fontSize: 12 };
  const profileCard = (selected: boolean, dimmed: boolean) => ({
    gap: 4,
    padding: 12,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: selected ? colors.accent : colors.border,
    backgroundColor: selected ? colors.surface2 : colors.surface1,
    opacity: dimmed ? 0.65 : 1,
  });
  const effortChip = (selected: boolean, dimmed: boolean) => ({
    paddingHorizontal: 12,
    paddingVertical: 8,
    borderRadius: 6,
    borderWidth: 1,
    borderColor: selected ? colors.accent : colors.border,
    backgroundColor: selected ? colors.surface2 : colors.surface0,
    opacity: dimmed ? 0.65 : 1,
  });
  const routeButton = (enabled: boolean) => ({
    padding: 12,
    borderRadius: 6,
    backgroundColor: colors.accent,
    opacity: enabled ? 1 : 0.55,
  });
  return {
    text,
    muted,
    title: { ...text, fontSize: 20, fontWeight: "600" as const },
    heading: { ...text, fontWeight: "600" as const },
    danger: { ...text, color: colors.statusDanger },
    mutedDanger: { ...muted, color: colors.statusDanger },
    scroll: { flex: 1, backgroundColor: colors.surface0 },
    content: { padding: compact ? 12 : 20, gap: 14 },
    input: {
      minHeight: 120,
      padding: 12,
      borderRadius: 8,
      borderWidth: 1,
      borderColor: colors.border,
      backgroundColor: colors.surface1,
      color: colors.foreground,
      textAlignVertical: "top" as const,
    },
    notice: { gap: 8 },
    secondaryButton: { padding: 10, borderRadius: 6, backgroundColor: colors.surface2 },
    profileCard: profileCard(false, false),
    profileCardDimmed: profileCard(false, true),
    profileCardSelected: profileCard(true, false),
    profileCardSelectedDimmed: profileCard(true, true),
    effortSection: { gap: 10 },
    effortCard: {
      gap: 8,
      padding: 12,
      borderRadius: 8,
      borderWidth: 1,
      borderColor: colors.border,
      backgroundColor: colors.surface1,
    },
    effortRow: { flexDirection: "row" as const, flexWrap: "wrap" as const, gap: 8 },
    effortChip: effortChip(false, false),
    effortChipDimmed: effortChip(false, true),
    effortChipSelected: effortChip(true, false),
    effortChipSelectedDimmed: effortChip(true, true),
    routeButton: routeButton(true),
    routeButtonDisabled: routeButton(false),
    routeLabel: {
      color: colors.accentForeground,
      fontWeight: "600" as const,
      textAlign: "center" as const,
    },
    outcomeCard: {
      gap: 6,
      padding: 12,
      borderRadius: 8,
      borderWidth: 1,
      borderColor: colors.border,
    },
  };
}

function profileCardStyle(styles: Styles, selected: boolean, dimmed: boolean) {
  if (selected) return dimmed ? styles.profileCardSelectedDimmed : styles.profileCardSelected;
  return dimmed ? styles.profileCardDimmed : styles.profileCard;
}

function effortChipStyle(styles: Styles, selected: boolean, dimmed: boolean) {
  if (selected) return dimmed ? styles.effortChipSelectedDimmed : styles.effortChipSelected;
  return dimmed ? styles.effortChipDimmed : styles.effortChip;
}

function buildModelGroups(profiles: ProfileView[], selectedProfileIds: string[]) {
  const groups = new Map<string, ModelGroup>();
  for (const profile of profiles) {
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
}

function buildAllowedModels(groups: ModelGroup[], selectedEfforts: Record<string, EffortId[]>) {
  return groups.map((group) => ({
    provider: group.provider,
    model: group.model,
    effortIds: (selectedEfforts[group.key] ?? []).filter((effortId) =>
      group.effortIds.includes(effortId),
    ),
  }));
}

function keepEffortsForProfiles(
  current: Record<string, EffortId[]>,
  profiles: ProfileView[],
  profileIds: string[],
) {
  const modelKeys = new Set(
    profiles
      .filter((candidate) => profileIds.includes(candidate.id))
      .map((candidate) => modelKey(candidate.provider, candidate.model)),
  );
  return Object.fromEntries(Object.entries(current).filter(([key]) => modelKeys.has(key)));
}

function toggledEfforts(currentEfforts: EffortId[], effortId: EffortId, selected: boolean) {
  return selected
    ? currentEfforts.filter((candidate) => candidate !== effortId)
    : effortIds.filter((candidate) => currentEfforts.includes(candidate) || candidate === effortId);
}

interface OutcomeInput {
  request: DirectInput | null;
  result: { agentId: string; selection: SelectionView } | undefined;
  records: unknown[] | undefined;
  runIsSuccess: boolean;
  runIsError: boolean;
  statusIsSuccess: boolean;
}

function deriveOutcome({
  request,
  result,
  records,
  runIsSuccess,
  runIsError,
  statusIsSuccess,
}: OutcomeInput) {
  const matchingRecord = request
    ? records
        ?.map(asDirectRecord)
        .find((record): record is DirectRecordView => record?.requestId === request.requestId)
    : undefined;
  const created = matchingRecord?.status === "created" ? matchingRecord : undefined;
  const agentId: string | undefined = result?.agentId ?? created?.agentId;
  const selection: SelectionView | undefined = result?.selection ?? created?.selection;
  const mayReset = runIsSuccess || ["created", "failed"].includes(matchingRecord?.status ?? "");
  const mayRetrySameRequest =
    runIsError && statusIsSuccess && !matchingRecord && records?.length === 0;
  return { matchingRecord, agentId, selection, mayReset, mayRetrySameRequest };
}

function routeLabel(isPending: boolean, mayRetrySameRequest: boolean) {
  if (isPending) return "Routing…";
  return mayRetrySameRequest ? "Retry same request" : "Route and run";
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
  const { refetch: refetchProfiles } = profiles;
  const { mutate: runMutate, reset: runReset } = run;
  const { mutate: statusMutate, reset: statusReset } = status;

  const styles = useMemo(
    () => createStyles(theme.colors, layout.compact),
    [theme.colors, layout.compact],
  );
  const profileList = profiles.data?.profiles;
  const locked = request !== null;
  const selectedModelGroups = useMemo(
    () => buildModelGroups(profileList ?? [], selectedProfileIds),
    [profileList, selectedProfileIds],
  );
  const selectedAllowedModels = useMemo(
    () => buildAllowedModels(selectedModelGroups, selectedEfforts),
    [selectedModelGroups, selectedEfforts],
  );
  const hasEffortForEveryModel =
    selectedAllowedModels.length > 0 &&
    selectedAllowedModels.every((model) => model.effortIds.length > 0);
  const { matchingRecord, agentId, selection, mayReset, mayRetrySameRequest } = deriveOutcome({
    request,
    result: run.data,
    records: status.data?.records,
    runIsSuccess: run.isSuccess,
    runIsError: run.isError,
    statusIsSuccess: status.isSuccess,
  });
  const canRoute =
    !run.isPending &&
    !status.isPending &&
    ((request === null &&
      prompt.trim().length >= 8 &&
      selectedProfileIds.length > 0 &&
      hasEffortForEveryModel) ||
      mayRetrySameRequest);

  const route = useCallback(() => {
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
    statusReset();
    runMutate(nextRequest);
  }, [
    request,
    workspaceId,
    prompt,
    selectedProfileIds,
    selectedAllowedModels,
    statusReset,
    runMutate,
  ]);

  const reset = useCallback(() => {
    setPrompt("");
    setSelectedProfileIds([]);
    setSelectedEfforts({});
    setRequest(null);
    runReset();
    statusReset();
  }, [runReset, statusReset]);

  const retryProfiles = useCallback(() => void refetchProfiles(), [refetchProfiles]);

  const checkStatus = useCallback(() => {
    if (request) statusMutate({ workspaceId, requestId: request.requestId });
  }, [request, statusMutate, workspaceId]);

  const toggleProfile = useCallback(
    (profileId: string) => {
      const nextProfileIds = selectedProfileIds.includes(profileId)
        ? selectedProfileIds.filter((id) => id !== profileId)
        : [...selectedProfileIds, profileId];
      setSelectedProfileIds(nextProfileIds);
      setSelectedEfforts((current) =>
        keepEffortsForProfiles(current, profileList ?? [], nextProfileIds),
      );
    },
    [profileList, selectedProfileIds],
  );

  const toggleEffort = useCallback((groupKey: string, effortId: EffortId, selected: boolean) => {
    setSelectedEfforts((current) => ({
      ...current,
      [groupKey]: toggledEfforts(current[groupKey] ?? [], effortId, selected),
    }));
  }, []);

  return (
    <ScrollView style={styles.scroll} contentContainerStyle={styles.content}>
      <Text style={styles.title}>Direct Jev routing</Text>
      <Text style={styles.muted}>
        Choose profiles, then explicitly allow effort levels for every selected model. Jev can only
        choose an allowed profile, model, and effort combination. Luna offers max effort only. The
        selected profile&apos;s mode and feature values stay unchanged. The current agent is not
        involved.
      </Text>

      <Text style={styles.heading}>Prompt</Text>
      <TextInput
        accessibilityLabel="Direct routing prompt"
        editable={!locked}
        maxLength={12000}
        multiline
        onChangeText={setPrompt}
        placeholder="Describe the task for the new agent"
        placeholderTextColor={theme.colors.foregroundMuted}
        style={styles.input}
        value={prompt}
      />

      <Text style={styles.heading}>Allowed profiles</Text>
      <Text style={styles.muted}>
        Select at least one profile. Jev only chooses from this list.
      </Text>
      <ProfileSection
        isError={profiles.isError}
        isFetching={profiles.isFetching}
        isPending={profiles.isPending}
        locked={locked}
        onRetry={retryProfiles}
        onToggle={toggleProfile}
        profiles={profileList}
        selectedProfileIds={selectedProfileIds}
        styles={styles}
      />

      <EffortSection
        groups={selectedModelGroups}
        locked={locked}
        onToggle={toggleEffort}
        selectedEfforts={selectedEfforts}
        styles={styles}
      />

      <Pressable
        accessibilityRole="button"
        accessibilityLabel={request ? "Retry the same direct routing request" : "Route and run"}
        disabled={!canRoute}
        onPress={route}
        style={canRoute ? styles.routeButton : styles.routeButtonDisabled}
      >
        <Text style={styles.routeLabel}>{routeLabel(run.isPending, mayRetrySameRequest)}</Text>
      </Pressable>

      {request ? (
        <Text selectable style={styles.muted}>
          Request {request.requestId}
        </Text>
      ) : null}
      {run.isError ? (
        <RunErrorNotice checking={status.isPending} onCheck={checkStatus} styles={styles} />
      ) : null}
      <StatusNotice
        isError={status.isError}
        isSuccess={status.isSuccess}
        matchingRecord={matchingRecord}
        styles={styles}
      />
      {selection ? (
        <AgentCreatedCard
          agentId={agentId}
          navigation={navigation}
          selection={selection}
          styles={styles}
        />
      ) : null}
      {mayReset ? (
        <Pressable accessibilityRole="button" onPress={reset} style={styles.secondaryButton}>
          <Text style={styles.text}>Reset for a new task</Text>
        </Pressable>
      ) : null}
    </ScrollView>
  );
}

interface ProfileSectionProps {
  isPending: boolean;
  isError: boolean;
  isFetching: boolean;
  profiles: ProfileView[] | undefined;
  selectedProfileIds: string[];
  locked: boolean;
  styles: Styles;
  onRetry: () => void;
  onToggle: (profileId: string) => void;
}

function ProfileSection({
  isPending,
  isError,
  isFetching,
  profiles,
  selectedProfileIds,
  locked,
  styles,
  onRetry,
  onToggle,
}: ProfileSectionProps) {
  return (
    <>
      {isPending ? <Text style={styles.muted}>Loading profiles…</Text> : null}
      {isError ? (
        <View style={styles.notice}>
          <Text accessibilityRole="alert" style={styles.danger}>
            Could not load profiles.
          </Text>
          <Pressable
            accessibilityRole="button"
            disabled={isFetching}
            onPress={onRetry}
            style={styles.secondaryButton}
          >
            <Text style={styles.text}>{isFetching ? "Retrying…" : "Retry profiles"}</Text>
          </Pressable>
        </View>
      ) : null}
      {profiles?.length === 0 ? (
        <Text style={styles.muted}>
          No direct-routing profiles are available for this workspace.
        </Text>
      ) : null}
      {profiles?.map((profile) => {
        const selected = selectedProfileIds.includes(profile.id);
        return (
          <ProfileRow
            key={profile.id}
            locked={locked}
            onToggle={onToggle}
            profile={profile}
            selected={selected}
            selectionLimitReached={!selected && selectedProfileIds.length >= 8}
            styles={styles}
          />
        );
      })}
    </>
  );
}

interface ProfileRowProps {
  profile: ProfileView;
  selected: boolean;
  selectionLimitReached: boolean;
  locked: boolean;
  styles: Styles;
  onToggle: (profileId: string) => void;
}

function ProfileRow({
  profile,
  selected,
  selectionLimitReached,
  locked,
  styles,
  onToggle,
}: ProfileRowProps) {
  const profileDisabled = offeredEffortIds(profile.model, profile.effortIds).length === 0;
  const disabled = locked || selectionLimitReached || profileDisabled;
  const accessibilityState = useMemo(() => ({ checked: selected, disabled }), [selected, disabled]);
  const handlePress = useCallback(() => onToggle(profile.id), [onToggle, profile.id]);
  return (
    <Pressable
      accessibilityRole="checkbox"
      accessibilityState={accessibilityState}
      disabled={disabled}
      onPress={handlePress}
      style={profileCardStyle(styles, selected, disabled)}
    >
      <Text style={styles.heading}>
        {selected ? "Selected · " : ""}
        {profile.name}
      </Text>
      <Text style={styles.muted}>
        {profile.provider} · {profile.model} · mode {profile.modeId ?? "default"}
      </Text>
      {profileDisabled ? (
        <Text style={styles.mutedDanger}>
          No effort levels are currently available for this profile.
        </Text>
      ) : null}
      {profile.notes ? <Text style={styles.text}>{profile.notes}</Text> : null}
    </Pressable>
  );
}

interface EffortSectionProps {
  groups: ModelGroup[];
  selectedEfforts: Record<string, EffortId[]>;
  locked: boolean;
  styles: Styles;
  onToggle: (groupKey: string, effortId: EffortId, selected: boolean) => void;
}

function EffortSection({ groups, selectedEfforts, locked, styles, onToggle }: EffortSectionProps) {
  if (groups.length === 0) return null;
  return (
    <View style={styles.effortSection}>
      <Text style={styles.heading}>Allowed model efforts</Text>
      <Text style={styles.muted}>
        Select at least one offered effort for each model. Only checked efforts are sent to Jev.
      </Text>
      {groups.map((group) => (
        <EffortGroupCard
          group={group}
          key={group.key}
          locked={locked}
          onToggle={onToggle}
          selectedEffortIds={selectedEfforts[group.key]}
          styles={styles}
        />
      ))}
    </View>
  );
}

interface EffortGroupCardProps {
  group: ModelGroup;
  selectedEffortIds: EffortId[] | undefined;
  locked: boolean;
  styles: Styles;
  onToggle: (groupKey: string, effortId: EffortId, selected: boolean) => void;
}

function EffortGroupCard({
  group,
  selectedEffortIds,
  locked,
  styles,
  onToggle,
}: EffortGroupCardProps) {
  return (
    <View style={styles.effortCard}>
      <Text style={styles.heading}>
        {group.provider} · {group.model}
      </Text>
      {group.profileCount > 1 ? (
        <Text style={styles.muted}>{group.profileCount} selected profiles use this model.</Text>
      ) : null}
      <View style={styles.effortRow}>
        {group.effortIds.map((effortId) => (
          <EffortChip
            effortId={effortId}
            groupKey={group.key}
            key={effortId}
            locked={locked}
            onToggle={onToggle}
            selected={selectedEffortIds?.includes(effortId) ?? false}
            styles={styles}
          />
        ))}
      </View>
    </View>
  );
}

interface EffortChipProps {
  groupKey: string;
  effortId: EffortId;
  selected: boolean;
  locked: boolean;
  styles: Styles;
  onToggle: (groupKey: string, effortId: EffortId, selected: boolean) => void;
}

function EffortChip({ groupKey, effortId, selected, locked, styles, onToggle }: EffortChipProps) {
  const accessibilityState = useMemo(
    () => ({ checked: selected, disabled: locked }),
    [selected, locked],
  );
  const handlePress = useCallback(
    () => onToggle(groupKey, effortId, selected),
    [onToggle, groupKey, effortId, selected],
  );
  return (
    <Pressable
      accessibilityRole="checkbox"
      accessibilityState={accessibilityState}
      disabled={locked}
      onPress={handlePress}
      style={effortChipStyle(styles, selected, locked)}
    >
      <Text style={styles.text}>
        {selected ? "Selected · " : ""}
        {effortId}
      </Text>
    </Pressable>
  );
}

interface RunErrorNoticeProps {
  checking: boolean;
  styles: Styles;
  onCheck: () => void;
}

function RunErrorNotice({ checking, styles, onCheck }: RunErrorNoticeProps) {
  return (
    <View style={styles.notice}>
      <Text accessibilityRole="alert" style={styles.danger}>
        The result is uncertain. Check the saved request before retrying or starting another task.
      </Text>
      <Pressable
        accessibilityRole="button"
        disabled={checking}
        onPress={onCheck}
        style={styles.secondaryButton}
      >
        <Text style={styles.text}>{checking ? "Checking…" : "Check request status"}</Text>
      </Pressable>
    </View>
  );
}

interface StatusNoticeProps {
  isError: boolean;
  isSuccess: boolean;
  matchingRecord: DirectRecordView | undefined;
  styles: Styles;
}

function StatusNotice({ isError, isSuccess, matchingRecord, styles }: StatusNoticeProps) {
  return (
    <>
      {isError ? (
        <Text accessibilityRole="alert" style={styles.danger}>
          Could not check the saved request. Try checking again with the same request ID.
        </Text>
      ) : null}
      {isSuccess && !matchingRecord ? (
        <Text style={styles.muted}>
          No saved record was found. A retry will reuse the same request ID.
        </Text>
      ) : null}
      {matchingRecord ? (
        <View style={styles.outcomeCard}>
          <Text style={styles.heading}>Saved request · {matchingRecord.status}</Text>
          {matchingRecord.error ? <Text style={styles.text}>{matchingRecord.error}</Text> : null}
          {matchingRecord.status === "interrupted" ? (
            <Text style={styles.muted}>
              This outcome is uncertain. Keep this request ID and inspect the saved agent labels.
            </Text>
          ) : null}
        </View>
      ) : null}
    </>
  );
}

interface AgentCreatedCardProps {
  selection: SelectionView;
  agentId: string | undefined;
  navigation: Navigation;
  styles: Styles;
}

function AgentCreatedCard({ selection, agentId, navigation, styles }: AgentCreatedCardProps) {
  const openAgent = useCallback(() => {
    if (agentId) navigation?.openAgent({ agentId });
  }, [agentId, navigation]);
  return (
    <View style={styles.outcomeCard}>
      <Text style={styles.heading}>Agent created</Text>
      <Text style={styles.text}>
        {selection.name ? `${selection.name} · ` : ""}
        {selection.model ?? "Unknown model"} · effort {selection.thinkingOptionId ?? "unknown"}
      </Text>
      {agentId && navigation ? (
        <Pressable accessibilityRole="button" onPress={openAgent} style={styles.secondaryButton}>
          <Text style={styles.text}>Open agent</Text>
        </Pressable>
      ) : null}
    </View>
  );
}
