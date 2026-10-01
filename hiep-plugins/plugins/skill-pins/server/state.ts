import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { savedSkillsSchema, skillsSchema, type SkillId } from "../shared/catalog";

const agentPattern = /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/i;

function ignoreMissing(error: NodeJS.ErrnoException) {
  if (error.code !== "ENOENT") throw error;
}

export class SkillPins {
  constructor(private root: string) {}
  private file(agentId: string) {
    if (!agentPattern.test(agentId)) throw new Error("Invalid agent ID");
    return path.join(this.root, "agents", agentId, "skills.json");
  }
  // Archived agents never send another turn, so their pins are dropped.
  // `file` rejects anything but a UUID, so this cannot reach outside `agents/`.
  async remove(agentId: string) {
    await rm(path.dirname(this.file(agentId)), { recursive: true, force: true });
  }
  async get(agentId: string): Promise<{ skills: SkillId[] }> {
    try {
      const value = JSON.parse(await readFile(this.file(agentId), "utf8"));
      return { skills: savedSkillsSchema.parse(value.skills) };
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return { skills: [] };
      throw error;
    }
  }
  // Writes the initial set only for an agent that has no selection yet.
  async seed(agentId: string, skills: readonly string[]) {
    try {
      await readFile(this.file(agentId));
    } catch (error) {
      ignoreMissing(error as NodeJS.ErrnoException);
      await this.set(agentId, skills);
    }
  }
  async set(agentId: string, skills: readonly string[]) {
    const value = { skills: skillsSchema.parse(skills) };
    const file = this.file(agentId);
    await mkdir(path.dirname(file), { recursive: true });
    const temporary = `${file}.${randomUUID()}.tmp`;
    await writeFile(temporary, JSON.stringify(value), { mode: 0o600 });
    await rename(temporary, file);
    return value;
  }
}
