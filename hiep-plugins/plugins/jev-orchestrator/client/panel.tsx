import { useRpc, type PluginAgentPanelProps } from "@getpaseo/plugin/client";
import { useQuery, useMutation } from "@tanstack/react-query";
import { useCallback, useMemo } from "react";
import { ScrollView, View, Text, Pressable } from "react-native";
import { jobsRpc, cancelRpc } from "../shared/contracts";

interface CheckView {
  exitCode: number | null;
  output: string;
}

interface AttemptView {
  childId: string;
  phase: string;
  profile: { name: string; model: string; thinkingOptionId?: string };
  status?: string;
  output?: string;
  checks?: CheckView[];
}

interface JobView {
  task: { id: string };
  status: string;
  message: string;
  notifyError?: string;
  attempts: AttemptView[];
}

type Colors = PluginAgentPanelProps["theme"]["colors"];
type Styles = ReturnType<typeof createStyles>;

function createStyles(colors: Colors, compact: boolean) {
  const text = { color: colors.foreground };
  return {
    text,
    title: { ...text, fontSize: 20, fontWeight: "600" as const },
    heading: { ...text, fontWeight: "600" as const },
    muted: { color: colors.foregroundMuted },
    button: { padding: 10, backgroundColor: colors.surface1, borderRadius: 6 },
    scroll: { flex: 1, backgroundColor: colors.surface0 },
    content: { padding: compact ? 12 : 20, gap: 16 },
    job: {
      gap: 8,
      borderColor: colors.border,
      borderWidth: 1,
      padding: 12,
      borderRadius: 8,
    },
    attempt: { gap: 4 },
  };
}

// Checks carry no id, so the key is their content plus a counter for identical repeats.
function keyedChecks(checks: CheckView[]) {
  const seen = new Map<string, number>();
  return checks.map((check, index) => {
    const content = `${check.exitCode}:${check.output}`;
    const repeat = seen.get(content) ?? 0;
    seen.set(content, repeat + 1);
    return { check, number: index + 1, key: `${content}:${repeat}` };
  });
}

export function JobsPanel({ agentId, theme, layout }: PluginAgentPanelProps) {
  const list = useRpc(jobsRpc),
    cancel = useRpc(cancelRpc);
  const query = useQuery({
    queryKey: ["jev-jobs", agentId],
    queryFn: () => list({ parentId: agentId }),
    refetchInterval: 5000,
  });
  const mutation = useMutation({
    mutationFn: (id: string) => cancel({ parentId: agentId, id }),
    onSuccess: () => {
      void query.refetch();
    },
  });
  const { refetch } = query;
  const { mutate } = mutation;
  const styles = useMemo(
    () => createStyles(theme.colors, layout.compact),
    [theme.colors, layout.compact],
  );
  const refresh = useCallback(() => {
    void refetch();
  }, [refetch]);
  return (
    <ScrollView style={styles.scroll} contentContainerStyle={styles.content}>
      <Text style={styles.title}>Delegated work</Text>
      <Text style={styles.muted}>
        Use delegate_task in a new agent to start. Checks, model judgments, and child claims remain
        separate.
      </Text>
      {(query.isError || mutation.isError) && (
        <Text style={styles.text}>
          Request failed. Refresh and inspect the job before retrying.
        </Text>
      )}
      {query.isPending && <Text style={styles.text}>Loading jobs…</Text>}
      {query.data?.jobs.length === 0 && (
        <Text style={styles.text}>No delegated jobs for this agent.</Text>
      )}
      {(query.data?.jobs as JobView[] | undefined)?.map((job) => (
        <JobCard
          cancelDisabled={mutation.isPending}
          job={job}
          key={job.task.id}
          onCancel={mutate}
          styles={styles}
        />
      ))}
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Refresh delegated jobs"
        style={styles.button}
        onPress={refresh}
      >
        <Text style={styles.text}>Refresh</Text>
      </Pressable>
    </ScrollView>
  );
}

interface JobCardProps {
  job: JobView;
  cancelDisabled: boolean;
  styles: Styles;
  onCancel: (id: string) => void;
}

function JobCard({ job, cancelDisabled, styles, onCancel }: JobCardProps) {
  const jobId = job.task.id;
  const handleCancel = useCallback(() => onCancel(jobId), [onCancel, jobId]);
  return (
    <View style={styles.job}>
      <Text style={styles.heading}>
        {job.task.id} · {job.status}
      </Text>
      <Text style={styles.text}>{job.message}</Text>
      {job.notifyError && <Text style={styles.text}>{job.notifyError}</Text>}
      {job.attempts.map((attempt) => (
        <AttemptCard attempt={attempt} key={attempt.childId} styles={styles} />
      ))}
      {["queued", "running", "needs_input", "interrupted"].includes(job.status) && (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`Cancel ${job.task.id}`}
          disabled={cancelDisabled}
          style={styles.button}
          onPress={handleCancel}
        >
          <Text style={styles.text}>Cancel and archive children</Text>
        </Pressable>
      )}
    </View>
  );
}

interface AttemptCardProps {
  attempt: AttemptView;
  styles: Styles;
}

function AttemptCard({ attempt: a, styles }: AttemptCardProps) {
  return (
    <View style={styles.attempt}>
      <Text selectable style={styles.text}>
        {a.phase} · {a.profile.name} · {a.profile.model} · effort{" "}
        {a.profile.thinkingOptionId ?? "default"}
      </Text>
      <Text selectable style={styles.muted}>
        {a.childId} · {a.status ?? "running"}
      </Text>
      {a.checks &&
        keyedChecks(a.checks).map(({ check, number, key }) => (
          <Text selectable key={key} style={styles.text}>
            Check {number}: exit {check.exitCode ?? "unknown"}
            {"\n"}
            {check.output.slice(-1200)}
          </Text>
        ))}
      {a.output && (
        <Text selectable style={styles.text}>
          {a.output.slice(-2500)}
        </Text>
      )}
    </View>
  );
}
