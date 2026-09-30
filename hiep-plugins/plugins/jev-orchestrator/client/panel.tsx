import { useRpc, type PluginAgentPanelProps } from "@getpaseo/plugin/client";
import { useQuery, useMutation } from "@tanstack/react-query";
import { ScrollView, View, Text, Pressable } from "react-native";
import { jobsRpc, cancelRpc } from "../shared/contracts";

type JobView = {
  task: { id: string };
  status: string;
  message: string;
  notifyError?: string;
  attempts: {
    childId: string;
    phase: string;
    profile: { name: string; model: string; thinkingOptionId?: string };
    status?: string;
    output?: string;
    checks?: { exitCode: number | null; output: string }[];
  }[];
};
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
  const text = { color: theme.colors.foreground };
  const button = { padding: 10, backgroundColor: theme.colors.surface1, borderRadius: 6 };
  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: theme.colors.surface0 }}
      contentContainerStyle={{ padding: layout.compact ? 12 : 20, gap: 16 }}
    >
      <Text style={{ ...text, fontSize: 20, fontWeight: "600" }}>Delegated work</Text>
      <Text style={{ color: theme.colors.foregroundMuted }}>
        Use delegate_task in a new agent to start. Checks, model judgments, and child claims remain
        separate.
      </Text>
      {(query.isError || mutation.isError) && (
        <Text style={text}>Request failed. Refresh and inspect the job before retrying.</Text>
      )}
      {query.isPending && <Text style={text}>Loading jobs…</Text>}
      {query.data?.jobs.length === 0 && <Text style={text}>No delegated jobs for this agent.</Text>}
      {(query.data?.jobs as JobView[] | undefined)?.map((job) => (
        <View
          key={job.task.id}
          style={{
            gap: 8,
            borderColor: theme.colors.border,
            borderWidth: 1,
            padding: 12,
            borderRadius: 8,
          }}
        >
          <Text style={{ ...text, fontWeight: "600" }}>
            {job.task.id} · {job.status}
          </Text>
          <Text style={text}>{job.message}</Text>
          {job.notifyError && <Text style={text}>{job.notifyError}</Text>}
          {job.attempts.map((a) => (
            <View key={a.childId} style={{ gap: 4 }}>
              <Text selectable style={text}>
                {a.phase} · {a.profile.name} · {a.profile.model} · effort{" "}
                {a.profile.thinkingOptionId ?? "default"}
              </Text>
              <Text selectable style={{ color: theme.colors.foregroundMuted }}>
                {a.childId} · {a.status ?? "running"}
              </Text>
              {a.checks?.map((check, i) => (
                <Text selectable key={i} style={text}>
                  Check {i + 1}: exit {check.exitCode ?? "unknown"}
                  {"\n"}
                  {check.output.slice(-1200)}
                </Text>
              ))}
              {a.output && (
                <Text selectable style={text}>
                  {a.output.slice(-2500)}
                </Text>
              )}
            </View>
          ))}
          {["queued", "running", "needs_input", "interrupted"].includes(job.status) && (
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={`Cancel ${job.task.id}`}
              disabled={mutation.isPending}
              style={button}
              onPress={() => mutation.mutate(job.task.id)}
            >
              <Text style={text}>Cancel and archive children</Text>
            </Pressable>
          )}
        </View>
      ))}
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Refresh delegated jobs"
        style={button}
        onPress={() => {
          void query.refetch();
        }}
      >
        <Text style={text}>Refresh</Text>
      </Pressable>
    </ScrollView>
  );
}
