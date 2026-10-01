import type { PaseoApi } from "@getpaseo/client";
import { mkdir, readFile, readdir, rename, unlink, writeFile } from "node:fs/promises";
import path from "node:path";

const MANIFEST = /^(claude|codex)-[0-9a-f]{24}\.json$/;
// Ticket slots use `jev-native-ticket-*` and are owned by NativeTickets, so they never match.
const DEFINITION = /^jev-native-[0-9a-f]{24}\.(md|toml)$/;
const NATIVE_PROVIDERS = new Set(["claude", "codex"]);

// Agent ID to the manifest file its session opened with; null records an open without one.
type Leases = Record<string, string | null>;
export interface LiveAgent {
  id: string;
  provider: string;
}

export async function listLiveAgents(paseo: Pick<PaseoApi, "agents">): Promise<LiveAgent[]> {
  const agents: LiveAgent[] = [];
  const cursors = new Set<string>();
  let cursor: string | undefined;
  for (;;) {
    const page = await paseo.agents.list({
      filter: { includeArchived: false },
      page: { limit: 200, ...(cursor ? { cursor } : {}) },
    });
    for (const { agent } of page.entries) agents.push({ id: agent.id, provider: agent.provider });
    if (!page.pageInfo.hasMore) return agents;
    cursor = page.pageInfo.nextCursor ?? undefined;
    if (!cursor || cursors.has(cursor)) throw new Error("Agent list changed during cleanup.");
    cursors.add(cursor);
  }
}

export class NativeCleanup {
  private queue: Promise<unknown> = Promise.resolve();
  private readonly leaseFile: string;
  // Launch order in this process, so a prune can keep agents opened after its agent list was read.
  private launches = 0;
  private readonly launchedAt = new Map<string, number>();

  constructor(
    private readonly base: string,
    private readonly agentDirectories: string[],
  ) {
    this.leaseFile = path.join(base, "leases.json");
  }

  // Launches and pruning share one queue so a prune never removes files a launch is writing.
  private run<T>(task: () => Promise<T>): Promise<T> {
    const next = this.queue.then(task, task);
    this.queue = next.catch(() => {});
    return next;
  }

  private async readLeases(): Promise<Leases> {
    try {
      return JSON.parse(await readFile(this.leaseFile, "utf8"));
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return {};
      throw error;
    }
  }

  private async writeLeases(leases: Leases) {
    await mkdir(this.base, { recursive: true });
    const temporary = `${this.leaseFile}.tmp`;
    await writeFile(temporary, JSON.stringify(leases, null, 2) + "\n", { mode: 0o600 });
    await rename(temporary, this.leaseFile);
  }

  launch(agentId: string, prepare: () => Promise<Record<string, string>>) {
    return this.run(async () => {
      const env = await prepare();
      const leases = await this.readLeases();
      const manifest = env.PASEO_JEV_NATIVE_POLICY;
      leases[agentId] = manifest ? path.basename(manifest) : null;
      await this.writeLeases(leases);
      this.launchedAt.set(agentId, ++this.launches);
      return env;
    });
  }

  // Records a session that loads no native definitions. A history or other non-native open
  // can run beside the agent's interactive session, so it never replaces that session's lease.
  // Claude and Codex agents get no null lease: one may still run an interactive session opened
  // before leases existed, and a lease would hide it from the keep-every-manifest check.
  open(agentId: string, provider: string) {
    return this.run(async () => {
      if (NATIVE_PROVIDERS.has(provider)) return;
      const leases = await this.readLeases();
      if (agentId in leases) return;
      leases[agentId] = null;
      await this.writeLeases(leases);
    });
  }

  // Call before reading the agent list passed to prune.
  mark() {
    return this.launches;
  }

  // The agent list is read outside the queue, so agents launched after `since` are kept
  // even when that list predates them.
  prune(live: LiveAgent[], since = this.launches) {
    return this.run(async () => {
      const liveIds = new Set(live.map(({ id }) => id));
      for (const [id, order] of this.launchedAt)
        if (order > since) liveIds.add(id);
        else if (!liveIds.has(id)) this.launchedAt.delete(id);
      const leases = Object.fromEntries(
        Object.entries(await this.readLeases()).filter(([id]) => liveIds.has(id)),
      );
      await this.writeLeases(leases);
      // Sessions opened before leases existed may still use any manifest; keep all until they end.
      const unknown = live.some(
        ({ id, provider }) => NATIVE_PROVIDERS.has(provider) && !(id in leases),
      );
      const keep = new Set(Object.values(leases).filter((name): name is string => !!name));
      // Agent folders are shared by every PASEO_HOME, so only files listed in this home's own
      // removed manifests are candidates; files of other homes are never listed here.
      const allowed = new Set(this.agentDirectories.map((directory) => path.resolve(directory)));
      const listed = async (file: string) => {
        const { definitions } = JSON.parse(await readFile(file, "utf8"));
        return (definitions as { path: string }[])
          .map((definition) => path.resolve(definition.path))
          .filter(
            (item) => allowed.has(path.dirname(item)) && DEFINITION.test(path.basename(item)),
          );
      };
      const removed: string[] = [];
      const candidates = new Set<string>();
      const referenced = new Set<string>();
      for (const name of await readdir(this.base)) {
        if (!MANIFEST.test(name)) continue;
        const file = path.join(this.base, name);
        if (unknown || keep.has(name)) {
          // An unreadable kept manifest could list anything, so it stops the whole prune.
          for (const item of await listed(file)) referenced.add(item);
          continue;
        }
        removed.push(file);
        try {
          for (const item of await listed(file)) candidates.add(item);
        } catch {
          // An unreadable old manifest is still removed; its files are left in place.
        }
      }
      for (const file of removed) await unlink(file);
      let definitions = 0;
      for (const item of candidates) {
        if (referenced.has(item)) continue;
        try {
          await unlink(item);
          definitions++;
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
        }
      }
      const manifests = removed.length;
      return { manifests, definitions, waitingForUnknownSessions: unknown };
    });
  }
}
