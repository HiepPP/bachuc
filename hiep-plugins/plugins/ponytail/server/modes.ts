import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { DEFAULT_MODE, ponytailModeSchema, type PonytailMode } from "../shared/modes";

export class AgentModes {
  constructor(private root: string) {}
  /** The per-agent data folder; the daemon hook writes `last-hook.json` here. */
  dir(agentId: string) {
    if (!/^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/i.test(agentId))
      throw new Error("Invalid agent ID");
    return path.join(this.root, "agents", agentId);
  }
  async write(file: string, value: unknown) {
    await mkdir(path.dirname(file), { recursive: true });
    const temporary = `${file}.${randomUUID()}.tmp`;
    await writeFile(temporary, JSON.stringify(value), { mode: 0o600 });
    await rename(temporary, file);
  }
  async get(agentId: string): Promise<{ mode: PonytailMode }> {
    return { mode: (await this.find(agentId)) ?? DEFAULT_MODE };
  }
  /** The saved mode, or null when the agent never saved one. */
  async find(agentId: string): Promise<PonytailMode | null> {
    try {
      const saved = JSON.parse(await readFile(path.join(this.dir(agentId), "mode.json"), "utf8"));
      return ponytailModeSchema.parse(saved.mode);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw error;
    }
  }
  async set(agentId: string, mode: PonytailMode) {
    const saved = { mode: ponytailModeSchema.parse(mode) };
    await this.write(path.join(this.dir(agentId), "mode.json"), saved);
    return saved;
  }
}
