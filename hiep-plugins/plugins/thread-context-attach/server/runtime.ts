import type { PaseoAgentHandle } from "@getpaseo/client";

// `closeRuntime` exists only on hosts with `server_info.features.agentRuntimeClose`.
type ClosableHandle = PaseoAgentHandle & { closeRuntime?: () => Promise<void> };

/**
 * Reading a timeline resumes the thread's provider processes. When the thread was `closed`
 * before the read, release them again, unless it started working in the meantime.
 */
export async function readWithoutKeepingRuntime<T>(
  handle: PaseoAgentHandle,
  statusBeforeRead: string,
  read: () => Promise<T>,
): Promise<T> {
  try {
    return await read();
  } finally {
    if (statusBeforeRead === "closed") await releaseRuntime(handle).catch(() => {});
  }
}

async function releaseRuntime(handle: ClosableHandle): Promise<void> {
  if (!handle.closeRuntime) return;
  const agent = (await handle.refresh())?.agent;
  if (!agent || agent.status !== "idle" || agent.pendingPermissions.length > 0) return;
  await handle.closeRuntime();
}
