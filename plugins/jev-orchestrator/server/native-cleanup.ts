import type { PaseoApi } from "@getpaseo/client";
import { readFile, readdir, rename, unlink, writeFile } from "node:fs/promises";
import path from "node:path";

const MANIFEST = /^(claude|codex)-[0-9a-f]{24}\.json$/;
// Ticket slots use `jev-native-ticket-*` and are owned by NativeTickets, so they never match.
const DEFINITION = /^jev-native-[0-9a-f]{24}\.(md|toml)$/;
const NATIVE_PROVIDERS = new Set(["claude", "codex"]);

// Agent ID to the manifest file its session opened with; null records an open without one.
type Leases = Record<string, string | null>;
export type LiveAgent = { id: string; provider: string };

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
      return env;
    });
  }

  prune(live: LiveAgent[]) {
    return this.run(async () => {
      const liveIds = new Set(live.map(({ id }) => id));
      const leases = Object.fromEntries(
        Object.entries(await this.readLeases()).filter(([id]) => liveIds.has(id)),
      );
      await this.writeLeases(leases);
      // Sessions opened before leases existed may still use any manifest; keep all until they end.
      const unknown = live.some(
        ({ id, provider }) => NATIVE_PROVIDERS.has(provider) && !(id in leases),
      );
      const keep = new Set(Object.values(leases).filter((name): name is string => !!name));
      let manifests = 0;
      const referenced = new Set<string>();
      for (const name of await readdir(this.base)) {
        if (!MANIFEST.test(name)) continue;
        const file = path.join(this.base, name);
        if (!unknown && !keep.has(name)) {
          await unlink(file);
          manifests++;
          continue;
        }
        // An unreadable kept manifest could reference anything, so skip definition removal.
        const { definitions } = JSON.parse(await readFile(file, "utf8"));
        for (const definition of definitions) referenced.add(path.basename(definition.path));
      }
      let definitions = 0;
      for (const directory of this.agentDirectories) {
        let names: string[];
        try {
          names = await readdir(directory);
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code === "ENOENT") continue;
          throw error;
        }
        for (const name of names) {
          if (!DEFINITION.test(name) || referenced.has(name)) continue;
          await unlink(path.join(directory, name));
          definitions++;
        }
      }
      return { manifests, definitions, waitingForUnknownSessions: unknown };
    });
  }
}
