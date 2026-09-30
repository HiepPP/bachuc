import { mkdir, readFile, readdir, rename, rm, unlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { createHash, randomUUID } from "node:crypto";
import { savedSkillsSchema, skillsSchema, type SkillId } from "../shared/catalog";

const agentPattern = /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/i;
const tokenPattern = /^\d+-[a-f0-9-]+$/i;
export const DRAFT_TTL = 10 * 60 * 1000;

export function digest(text: string) {
  return createHash("sha256").update(text.replace(/\r\n/g, "\n")).digest("hex");
}

function ignoreMissing(error: NodeJS.ErrnoException) {
  if (error.code !== "ENOENT") throw error;
}

export class SkillPins {
  constructor(private root: string) {}
  private dir(agentId: string) {
    if (!agentPattern.test(agentId)) throw new Error("Invalid agent ID");
    return path.join(this.root, "agents", agentId);
  }
  private pending(agentId: string, token: string, extension: "json" | "queue") {
    if (!tokenPattern.test(token)) throw new Error("Invalid turn token");
    return path.join(this.dir(agentId), "pending", `${token}.${extension}`);
  }
  private async write(file: string, value: unknown) {
    await mkdir(path.dirname(file), { recursive: true });
    const temporary = `${file}.${randomUUID()}.tmp`;
    await writeFile(temporary, JSON.stringify(value), { mode: 0o600 });
    await rename(temporary, file);
  }
  // Archived agents never send another turn, so their pins and snapshots are dropped.
  // `dir` rejects anything but a UUID, so this cannot reach outside `agents/`.
  async remove(agentId: string) {
    await rm(this.dir(agentId), { recursive: true, force: true });
  }
  async get(agentId: string): Promise<{ skills: SkillId[] }> {
    try {
      const value = JSON.parse(await readFile(path.join(this.dir(agentId), "skills.json"), "utf8"));
      return { skills: savedSkillsSchema.parse(value.skills) };
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return { skills: [] };
      throw error;
    }
  }
  private get draftFile() {
    return path.join(this.root, "draft.json");
  }
  async writeDraft(skills: readonly string[]) {
    const value = { skills: skillsSchema.parse(skills) };
    await this.write(this.draftFile, { ...value, createdAt: Date.now() });
    return value;
  }
  // Returns null when there is no draft or it expired, so callers fall back to the defaults.
  async readDraft(): Promise<SkillId[] | null> {
    try {
      const value = JSON.parse(await readFile(this.draftFile, "utf8"));
      if (Date.now() - value.createdAt <= DRAFT_TTL) return savedSkillsSchema.parse(value.skills);
      await unlink(this.draftFile).catch(ignoreMissing);
    } catch (error) {
      if (!(error instanceof SyntaxError)) ignoreMissing(error as NodeJS.ErrnoException);
    }
    return null;
  }
  // A draft applies to one created agent only.
  async takeDraft() {
    const skills = await this.readDraft();
    await unlink(this.draftFile).catch(ignoreMissing);
    return skills;
  }
  // Writes the initial set only for an agent that has no selection yet.
  async seed(agentId: string, skills: readonly string[]) {
    try {
      await readFile(path.join(this.dir(agentId), "skills.json"));
    } catch (error) {
      ignoreMissing(error as NodeJS.ErrnoException);
      await this.set(agentId, skills);
    }
  }
  async set(agentId: string, skills: readonly string[]) {
    const value = { skills: skillsSchema.parse(skills) };
    await this.write(path.join(this.dir(agentId), "skills.json"), value);
    return value;
  }
  // The snapshot keeps only a hash, so a queued prompt keeps the skills chosen at send time
  // without storing the prompt text.
  async prepare(input: { agentId: string; text: string; skills: readonly string[] }) {
    const { skills } = await this.set(input.agentId, input.skills);
    const token = `${Date.now()}-${randomUUID()}`;
    await this.write(this.pending(input.agentId, token, "json"), {
      hash: digest(input.text),
      skills,
      createdAt: Date.now(),
    });
    return { token };
  }
  async bindQueue(agentId: string, token: string, queueId: string) {
    // A queue row seen again (for example after switching agents) keeps its first snapshot.
    if (await this.boundTokens(agentId, queueId).then((tokens) => tokens.length)) {
      await this.cancel(agentId, token);
      return {};
    }
    await this.write(this.pending(agentId, token, "queue"), { queueId });
    try {
      await readFile(this.pending(agentId, token, "json"));
    } catch (error) {
      ignoreMissing(error as NodeJS.ErrnoException);
      // The hook already consumed the snapshot, so the new binding is orphaned.
      await this.cancel(agentId, token);
    }
    return {};
  }
  private async boundTokens(agentId: string, queueId: string) {
    const dir = path.join(this.dir(agentId), "pending");
    const names = await readdir(dir).catch((error) => {
      ignoreMissing(error);
      return [] as string[];
    });
    const tokens: string[] = [];
    for (const name of names) {
      if (!name.endsWith(".queue")) continue;
      try {
        const value = JSON.parse(await readFile(path.join(dir, name), "utf8"));
        if (value.queueId === queueId) tokens.push(name.slice(0, -".queue".length));
      } catch (error) {
        ignoreMissing(error as NodeJS.ErrnoException);
      }
    }
    return tokens;
  }
  async cancelQueue(agentId: string, queueId: string) {
    for (const token of await this.boundTokens(agentId, queueId)) await this.cancel(agentId, token);
    return {};
  }
  async cancel(agentId: string, token: string) {
    await unlink(this.pending(agentId, token, "json")).catch(ignoreMissing);
    await unlink(this.pending(agentId, token, "queue")).catch(ignoreMissing);
    return {};
  }
}
