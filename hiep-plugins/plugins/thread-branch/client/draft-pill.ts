import type {
  PluginButtonContext,
  PluginButtonLabelProps,
  PluginClientContext,
} from "@getpaseo/plugin/client";
import { copyText } from "@getpaseo/plugin/client/react-native";
import { useEffect, useSyncExternalStore } from "react";
import { getBranchRpc, type BranchInfo } from "../shared/branch";
import { draftPillLabel } from "./label";
import { POLL_MS } from "./pills";

type Client = Pick<PluginClientContext, "rpc" | "addComposerPill">;

function draftCwd(context: PluginButtonContext | undefined) {
  return context?.context === "draft" ? context.draft.cwd : undefined;
}

/** One branch pill on New workspace, naming the branch of the project the draft has selected. */
export function installDraftBranchPill(
  client: Client,
  deps: { copy?: (text: string) => Promise<void>; intervalMs?: number } = {},
) {
  const copy = deps.copy ?? copyText;
  const intervalMs = deps.intervalMs ?? POLL_MS;
  // The label and the menu actions run apart; this keeps both on the last read of each directory.
  const infoByCwd = new Map<string, BranchInfo>();
  const listeners = new Set<() => void>();
  let stopped = false;

  function subscribe(listener: () => void) {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }

  async function load(cwd: string, mode?: "force" | "fetch") {
    const input =
      mode === "fetch" ? { cwd, fetch: true } : mode === "force" ? { cwd, force: true } : { cwd };
    const info = await client.rpc(getBranchRpc, input);
    if (!stopped) {
      infoByCwd.set(cwd, info);
      for (const listener of listeners) listener();
    }
    return info;
  }

  function requireCwd(context: PluginButtonContext | undefined) {
    const cwd = draftCwd(context);
    if (!cwd) throw new Error("Choose a project first.");
    return cwd;
  }

  function DraftBranchLabel(props: PluginButtonLabelProps) {
    const cwd = draftCwd(props);
    const info = useSyncExternalStore(subscribe, () => (cwd ? infoByCwd.get(cwd) : undefined));
    useEffect(() => {
      if (!cwd) return;
      const tick = () => void load(cwd).catch(() => {});
      tick();
      const timer = setInterval(tick, intervalMs);
      return () => clearInterval(timer);
    }, [cwd]);
    return draftPillLabel(cwd, info);
  }

  const pill = client.addComposerPill({
    id: "thread-branch-draft",
    showOnDraft: "only",
    button: {
      title: "Git branch of the selected project",
      icon: "GitBranch",
      label: DraftBranchLabel,
      behavior: {
        kind: "menu",
        items: [
          {
            kind: "item",
            id: "copy-branch",
            title: "Copy branch name",
            icon: "Copy",
            behavior: {
              kind: "action",
              async onPress(context) {
                const cwd = requireCwd(context);
                const info = infoByCwd.get(cwd) ?? (await load(cwd));
                if (info.detached || !info.branch) throw new Error("No branch name to copy.");
                await copy(info.branch);
              },
            },
          },
          { kind: "separator", id: "refresh-divider" },
          {
            kind: "item",
            id: "fetch",
            title: "Fetch",
            icon: "CloudDownload",
            behavior: {
              kind: "action",
              onPress: async (context) => void (await load(requireCwd(context), "fetch")),
            },
          },
          {
            kind: "item",
            id: "refresh",
            title: "Refresh",
            icon: "RefreshCw",
            behavior: {
              kind: "action",
              onPress: async (context) => void (await load(requireCwd(context), "force")),
            },
          },
        ],
      },
    },
  });

  return () => {
    stopped = true;
    pill.remove();
    infoByCwd.clear();
    listeners.clear();
  };
}
