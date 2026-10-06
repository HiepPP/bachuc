import type { Agent } from "@/stores/session-store";

export type CloseAgentTabPolicy = { kind: "archive-on-close" } | { kind: "layout-only" };

export function resolveCloseAgentTabPolicy(
  agent: Pick<Agent, "parentAgentId"> | null | undefined,
  options: { fromShortcut?: boolean } = {},
): CloseAgentTabPolicy {
  // Cmd+W is easy to press by mistake, so a shortcut only closes the tab.
  if (agent?.parentAgentId || options.fromShortcut) {
    return { kind: "layout-only" };
  }

  return { kind: "archive-on-close" };
}
