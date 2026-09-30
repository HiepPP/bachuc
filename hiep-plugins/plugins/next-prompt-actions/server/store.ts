import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import path from "node:path";
import { z } from "zod";

const entrySchema = z.object({
  enabled: z.boolean(),
  handled: z.record(z.string(), z.enum(["sending", "sent", "unknown"])),
  evaluated: z.array(z.string()),
  remaining: z.number().int().min(0).max(3),
  scope: z.string(),
  autoMessageIds: z.array(z.string()),
});
const dataSchema = z.record(z.string(), entrySchema);
export type Entry = z.infer<typeof entrySchema>;
export class Store {
  readonly data: Record<string, Entry>;
  constructor(private file?: string) {
    this.data =
      file && existsSync(file) ? dataSchema.parse(JSON.parse(readFileSync(file, "utf8"))) : {};
    for (const state of Object.values(this.data))
      for (const key of Object.keys(state.handled))
        if (state.handled[key] === "sending") state.handled[key] = "unknown";
  }
  get(id: string): Entry {
    return (this.data[id] ??= {
      enabled: false,
      handled: {},
      evaluated: [],
      remaining: 0,
      scope: "",
      autoMessageIds: [],
    });
  }
  save() {
    if (!this.file) return;
    mkdirSync(path.dirname(this.file), { recursive: true, mode: 0o700 });
    writeFileSync(this.file + ".tmp", JSON.stringify(this.data), { mode: 0o600 });
    renameSync(this.file + ".tmp", this.file);
  }
}
