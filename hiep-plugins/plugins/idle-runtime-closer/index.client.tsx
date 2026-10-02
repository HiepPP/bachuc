import type { PluginClientContext } from "@getpaseo/plugin/client";

export default function contribute(client: PluginClientContext) {
  const pill = client.addComposerPill({
    id: "close-runtime",
    button: {
      title: "Dọn process; giữ conversation. Công việc nền và trạng thái MCP trong RAM sẽ mất.",
      label: "Dọn process",
      icon: "Power",
      // Hidden for now: Board's Remove actions close the runtime. Drop this line to show it again.
      visible: false,
      behavior: {
        kind: "action",
        async onPress(context) {
          if (context?.context !== "agent") return;
          const agent = client.paseo.agents.ref(context.agentId);
          const snapshot = await agent.refresh();
          if (!snapshot) throw new Error("Không tìm thấy thread.");
          if (
            snapshot.agent.status === "running" ||
            snapshot.agent.status === "initializing" ||
            snapshot.agent.pendingPermissions.length > 0
          ) {
            throw new Error("Chờ thread hoàn thành và xử lý quyền đang chờ trước khi dọn process.");
          }
          await agent.closeRuntime();
        },
      },
    },
  });
  return () => pill.remove();
}
