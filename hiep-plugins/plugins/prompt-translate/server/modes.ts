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
  async prefs(agentId: string, defaultRewrite: boolean): Promise<{ mode: Mode; rewrite: boolean }> {
    const saved = await this.read(agentId);
    return {
      mode: saved ? cavemanModeSchema.parse(saved.mode) : "follow-agent",
      rewrite: typeof saved?.rewrite === "boolean" ? saved.rewrite : defaultRewrite,
    };
  }
  // Merges so a mode change keeps the rewrite choice and the reverse.
  async update(agentId: string, patch: { mode?: Mode; rewrite?: boolean }) {
    const saved = (await this.read(agentId)) ?? { mode: "follow-agent" };
    const next = {
      ...saved,
      ...(patch.mode ? { mode: cavemanModeSchema.parse(patch.mode) } : {}),
      ...(patch.rewrite === undefined ? {} : { rewrite: patch.rewrite }),
    };
    await this.write(path.join(this.dir(agentId), "mode.json"), next);
    return next;
  }
  async set(agentId: string, mode: Mode) {
    await this.update(agentId, { mode });
    return { mode };
  }
  private async read(agentId: string): Promise<{ mode: unknown; rewrite?: unknown } | null> {
    try {
      return JSON.parse(await readFile(path.join(this.dir(agentId), "mode.json"), "utf8"));
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw error;
    }
  }
}
