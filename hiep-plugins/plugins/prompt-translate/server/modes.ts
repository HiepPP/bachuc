import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { cavemanModeSchema, type TranslateSettings } from "../shared/settings";

type Mode = TranslateSettings["cavemanMode"];
export class AgentModes {
  constructor(private root: string) {}
  private dir(agentId: string) {
    if (!/^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/i.test(agentId))
      throw new Error("Invalid agent ID");
    return path.join(this.root, "agents", agentId);
  }
  private async write(file: string, value: unknown) {
    await mkdir(path.dirname(file), { recursive: true });
    const temporary = `${file}.${randomUUID()}.tmp`;
    await writeFile(temporary, JSON.stringify(value), { mode: 0o600 });
    await rename(temporary, file);
  }
  async get(agentId: string): Promise<{ mode: Mode }> {
    try {
      return {
        mode: cavemanModeSchema.parse(
          JSON.parse(await readFile(path.join(this.dir(agentId), "mode.json"), "utf8")).mode,
        ),
      };
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return { mode: "follow-agent" };
      throw error;
    }
  }
  /** The saved mode, or null when the agent never saved one. */
  async find(agentId: string): Promise<Mode | null> {
    const saved = await this.read(agentId);
    return saved ? cavemanModeSchema.parse(saved.mode) : null;
  }
  async set(agentId: string, mode: Mode) {
    const saved = { mode: cavemanModeSchema.parse(mode) };
    await this.write(path.join(this.dir(agentId), "mode.json"), saved);
    return saved;
  }
  // Files written while the pill had a rewrite switch also hold `rewrite`; only `mode` is read.
  private async read(agentId: string): Promise<{ mode: unknown } | null> {
    try {
      return JSON.parse(await readFile(path.join(this.dir(agentId), "mode.json"), "utf8"));
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw error;
    }
  }
}
