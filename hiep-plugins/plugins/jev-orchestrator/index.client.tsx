import type { PluginClientContext } from "@getpaseo/plugin/client";
import { DirectPanel } from "./client/direct-panel";
import { JobsPanel } from "./client/panel";
import { z } from "zod";
import { useMemo } from "react";
import { Text, View } from "react-native";

const jobItemStyle = { padding: 12, gap: 6 };

interface JobTimelineItemProps {
  item: { data: { id: string; status: string; message: string } };
  theme: { colors: { foreground: string; foregroundMuted: string } };
}

function JobTimelineItem({ item, theme }: JobTimelineItemProps) {
  const { foreground, foregroundMuted } = theme.colors;
  const titleStyle = useMemo(
    () => ({ color: foreground, fontWeight: "600" as const }),
    [foreground],
  );
  const messageStyle = useMemo(() => ({ color: foregroundMuted }), [foregroundMuted]);
  return (
    <View style={jobItemStyle}>
      <Text style={titleStyle}>
        {item.data.id} · {item.data.status}
      </Text>
      <Text style={messageStyle}>{item.data.message}</Text>
    </View>
  );
}

export default function contribute(client: PluginClientContext) {
  const jobsPanel = client.addWorkspacePanel({
    id: "jobs",
    title: "Delegated work",
    icon: "GitBranch",
    context: "agent",
    Component: JobsPanel,
  });
  const jobsCommand = client.addSlashCommand({
    argumentHint: "",
    name: "delegations",
    description: "Open autonomous delegation jobs and evidence",
    context: "agent",
    onSubmit: ({ openPanel }) => {
      openPanel("jobs");
    },
  });
  const directPanel = client.addWorkspacePanel({
    id: "direct-route",
    title: "Direct route",
    icon: "Route",
    context: "workspace",
    locations: ["workspace", "explorer"],
    Component: DirectPanel,
  });
  const directCommand = client.addCommandCenterItem({
    id: "open-direct-route",
    title: "Open direct Jev routing",
    icon: "Route",
    keywords: ["model", "effort", "agent"],
    context: "workspace",
    onSelect: ({ openPanel }) => openPanel("direct-route"),
  });
  const directSlash = client.addSlashCommand({
    argumentHint: "",
    name: "route",
    description: "Open direct Jev model and effort routing",
    context: "workspace",
    onSubmit: ({ args, openPanel }) => {
      if (args) throw new Error("Use /route without arguments.");
      openPanel("direct-route");
    },
  });
  const renderer = client.addTimelineRenderer({
    kind: "job",
    version: 1,
    schema: z.object({
      id: z.string(),
      status: z.string(),
      message: z.string(),
      children: z.array(z.string()),
    }),
    Component: JobTimelineItem,
  });
  return () => {
    renderer();
    directSlash();
    directCommand();
    directPanel();
    jobsCommand();
    jobsPanel();
  };
}
