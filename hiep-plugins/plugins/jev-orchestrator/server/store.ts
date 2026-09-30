import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { Job, Metric } from "./types";

export class Store {
  jobs: Job[] = [];
  metrics: Metric[] = [];
  constructor(private file?: string) {
    if (!file) return;
    try {
      const data = JSON.parse(readFileSync(file, "utf8"));
      if (data.version !== 1 || !Array.isArray(data.jobs) || !Array.isArray(data.metrics))
        throw new Error("Invalid orchestrator state.");
      this.jobs = data.jobs;
      this.metrics = data.metrics;
      for (const job of this.jobs)
        if (["running", "queued"].includes(job.status)) {
          job.status = "interrupted";
          job.message =
            "Plugin restarted. Inspect recorded children, then cancel before resubmitting. No automatic replay.";
        }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
  }
  save() {
    this.metrics = this.metrics.slice(-500);
    if (!this.file) return;
    mkdirSync(path.dirname(this.file), { recursive: true, mode: 0o700 });
    const temporary = this.file + ".tmp";
    writeFileSync(
      temporary,
      JSON.stringify({ version: 1, jobs: this.jobs, metrics: this.metrics }),
      { mode: 0o600 },
    );
    renameSync(temporary, this.file);
  }
}
