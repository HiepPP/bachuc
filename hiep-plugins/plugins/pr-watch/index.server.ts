import type { PaseoApi } from "@getpaseo/client";
import type { PluginServerContext } from "@getpaseo/plugin/server";
import { homedir } from "node:os";
import path from "node:path";
import { z } from "zod";
import type { PrSnapshot } from "./server/diff";
import { readPullRequest, readViewerLogin } from "./server/gh";
import { createWatchStore } from "./server/store";
import { createWatcher, POLL_INTERVAL_MS } from "./server/watcher";

const TICK_MS = 30_000;

function summarize(snapshot: PrSnapshot): string {
  const count = { passed: 0, failed: 0, pending: 0 };
  for (const check of snapshot.checks) count[check.result]++;
  const checks =
    snapshot.checks.length === 0
      ? "no checks"
      : `${count.passed} passed, ${count.failed} failed, ${count.pending} pending`;
  return `PR #${snapshot.number} (${snapshot.url}): ${checks}; mergeable ${snapshot.mergeable}.`;
}

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const store = createWatchStore(path.join(home, "plugin-data/pr-watch/watches.json"));
  const log = (line: string) => console.warn(`[pr-watch] ${line}`);
  const ready = store
    .load()
    .catch((error: unknown) => log(`could not load watches: ${String(error)}`));

  // Timers have no host context, so the poll loop uses the last `paseo` a hook or tool
  // handed over. Saved watches resume once any agent turn or tool call supplies one.
  let paseo: PaseoApi | null = null;
  let viewerLogin: Promise<string | null> | null = null;

  const watcher = createWatcher({
    store,
    readPr: (cwd, number) => readPullRequest(cwd, number),
    viewerLogin: (cwd) => (viewerLogin ??= readViewerLogin(cwd)),
    async isAgentBusy(agentId) {
      if (!paseo) return false;
      const handle = paseo.agents.ref(agentId);
      await handle.refresh();
      return handle.status === "running";
    },
    async wake(agentId, prompt) {
      if (!paseo) throw new Error("No Paseo connection to wake the agent yet.");
      await paseo.agents.ref(agentId).send(prompt);
    },
    now: () => Date.now(),
    log,
  });

  const timer = setInterval(() => {
    if (!paseo) return;
    void ready.then(() => watcher.tick()).catch((error: unknown) => log(String(error)));
  }, TICK_MS);

  const removeStarted = server.on("agent.turn_started", (_event, context) => {
    paseo = context.paseo;
  });
  const removeEnded = server.on("agent.turn_ended", ({ agent }, context) => {
    paseo = context.paseo;
    void watcher.flushHeld(agent.id).catch((error: unknown) => log(String(error)));
  });
  const removeArchived = server.on("agent.archived", ({ agent }, context) => {
    paseo = context.paseo;
    watcher.forget(agent.id);
    void ready.then(() => store.remove(agent.id)).catch((error: unknown) => log(String(error)));
  });

  server.registerTool({
    name: "watch_pull_request",
    description: `Watch a GitHub pull request for you. Every ${POLL_INTERVAL_MS / 60_000} minutes Paseo checks it with gh and sends you a prompt when checks fail, all checks pass, someone else comments or reviews, a merge conflict appears, or the PR is merged or closed. A prompt waits until your current turn ends. Omit number to watch the PR of the branch checked out in your working directory.`,
    inputSchema: z.object({ number: z.number().int().positive().optional() }),
    annotations: {
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: true,
    },
    async handler({ number }, context) {
      paseo = context.paseo;
      if (!context.callerAgentId) return { text: "PR watch needs a calling agent.", isError: true };
      await ready;
      const handle = context.paseo.agents.ref(context.callerAgentId);
      await handle.refresh();
      const cwd = handle.cwd;
      if (!cwd) return { text: "Could not find your working directory.", isError: true };
      let snapshot: PrSnapshot;
      try {
        snapshot = await readPullRequest(cwd, number ?? null);
      } catch (error) {
        const reason = error instanceof Error ? error.message : String(error);
        return { text: `Could not read the pull request: ${reason}`, isError: true };
      }
      if (snapshot.state !== "OPEN") {
        return {
          text: `PR #${snapshot.number} is already ${snapshot.state.toLowerCase()}.`,
          isError: true,
        };
      }
      const watch = {
        agentId: context.callerAgentId,
        cwd,
        number: snapshot.number,
        url: snapshot.url,
        createdAt: new Date().toISOString(),
        snapshot,
        failedReads: 0,
      };
      await store.put(watch);
      watcher.schedule(watch);
      return {
        text: `Watching ${summarize(snapshot)}`,
        structured: { number: snapshot.number, url: snapshot.url },
      };
    },
  });

  server.registerTool({
    name: "unwatch_pull_request",
    description: "Stop watching a pull request. Omit number to stop all of your PR watches.",
    inputSchema: z.object({ number: z.number().int().positive().optional() }),
    annotations: {
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    },
    async handler({ number }, context) {
      paseo = context.paseo;
      if (!context.callerAgentId) return { text: "PR watch needs a calling agent.", isError: true };
      await ready;
      const removed = await store.remove(context.callerAgentId, number);
      const list = removed.map((watch) => `#${watch.number}`).join(", ");
      return { text: removed.length > 0 ? `Stopped watching ${list}.` : "No matching PR watch." };
    },
  });

  server.registerTool({
    name: "list_pull_request_watches",
    description: "List the pull requests Paseo is watching for you.",
    inputSchema: z.object({}),
    annotations: {
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    },
    async handler(_input, context) {
      paseo = context.paseo;
      if (!context.callerAgentId) return { text: "PR watch needs a calling agent.", isError: true };
      await ready;
      const watches = store.listForAgent(context.callerAgentId);
      if (watches.length === 0) return { text: "You are not watching any pull request." };
      return { text: watches.map((watch) => `Watching ${summarize(watch.snapshot)}`).join("\n") };
    },
  });

  return () => {
    clearInterval(timer);
    removeStarted();
    removeEnded();
    removeArchived();
  };
}
