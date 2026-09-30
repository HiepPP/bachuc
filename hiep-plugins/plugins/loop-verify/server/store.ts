import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import path from "node:path";
import { z } from "zod";

const loopSchema = z.object({
  rootId: z.string(),
  current: z.string(),
  round: z.number().int().min(1),
  maxRounds: z.number().int().min(1),
  verify: z.string(),
  goal: z.string().nullable(),
  cwd: z.string(),
  config: z.object({
    provider: z.string(),
    modeId: z.string().optional(),
    thinkingOptionId: z.string().optional(),
  }),
  status: z.enum(["running", "paused", "passed", "exhausted", "stopped"]),
  history: z.array(
    z.object({ agentId: z.string(), round: z.number().int(), exitCode: z.number().nullable() }),
  ),
});
const dataSchema = z.record(z.string(), loopSchema);
export type Loop = z.infer<typeof loopSchema>;

export class Store {
  private readonly data: Record<string, Loop>;

  constructor(private readonly file?: string) {
    this.data =
      file && existsSync(file) ? dataSchema.parse(JSON.parse(readFileSync(file, "utf8"))) : {};
  }

  active(agentId: string): Loop | undefined {
    return Object.values(this.data).find(
      (loop) => (loop.status === "running" || loop.status === "paused") && loop.current === agentId,
    );
  }

  byRoot(rootId: string): Loop | undefined {
    return this.data[rootId];
  }

  put(loop: Loop): void {
    this.data[loop.rootId] = loop;
    this.save();
  }

  save(): void {
    if (!this.file) return;
    mkdirSync(path.dirname(this.file), { recursive: true, mode: 0o700 });
    writeFileSync(this.file + ".tmp", JSON.stringify(this.data, null, 2), { mode: 0o600 });
    renameSync(this.file + ".tmp", this.file);
  }
}
