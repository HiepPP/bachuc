import type { PluginClientContext } from "@getpaseo/plugin/client";
import { PreflightPanel } from "./client/panel";

export default function contribute(client: PluginClientContext) {
  const panel = client.addWorkspacePanel({
    id: "report",
    title: "Preflight",
    icon: "ClipboardCheck",
    context: "workspace",
    locations: ["explorer"],
    Component: PreflightPanel,
  });
  const command = client.addCommandCenterItem({
    id: "open-preflight",
    title: "Open workspace preflight",
    icon: "ClipboardCheck",
    context: "workspace",
    onSelect: ({ openPanel }) => openPanel("report", { location: "explorer" }),
  });
  const slash = client.addSlashCommand({
    name: "preflight",
    description: "Check workspace readiness without repairs",
    argumentHint: "",
    context: "workspace",
    onSubmit: ({ args, openPanel }) => {
      if (args) throw new Error("Use /preflight without arguments.");
      openPanel("report", { location: "explorer" });
    },
  });
  return () => {
    slash();
    command();
    panel();
  };
}
