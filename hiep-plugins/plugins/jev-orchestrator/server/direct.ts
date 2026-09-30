import type { PaseoApi, PaseoProviderModelsResult } from "@getpaseo/client";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { realpath } from "node:fs/promises";
import path from "node:path";
import {
  directInput,
  effortSchema,
  isLunaModel,
  type ModelAllowlist,
  type DirectInput,
} from "../shared/direct";
import { createDriver } from "./paseo";
import type { Judge, Profile } from "./types";
import { missingUsage, type TokenUsage } from "./usage";

type Selection = Profile & { thinkingOptionId: string };
type Candidate = Selection & { profileId: string };
export type DirectResult = {
  agentId: string;
  selection: Selection;
  routingMs: number;
  usage: TokenUsage;
};
export type DirectRecord = {
  requestId: string;
  workspaceId: string;
  fingerprint: string;
  status: "routing" | "creating" | "created" | "failed" | "interrupted";
  startedAt: number;
  endedAt?: number;
  routingMs?: number;
  selection?: Selection;
  agentId?: string;
  usage: TokenUsage;
  error?: string;
};

export function expandCandidates(
  profiles: Profile[],
  catalogs: Map<string, PaseoProviderModelsResult>,
  allowedModels: ModelAllowlist,
): Candidate[] {
  const candidates: Candidate[] = [];
  for (const profile of profiles) {
    const catalog = catalogs.get(profile.provider);
    const model = catalog?.models?.find(
      (m) => m.id === profile.model || m.aliases?.includes(profile.model),
    );
    const allowed = allowedModels.find(
      (entry) => entry.provider === profile.provider && entry.model === profile.model,
    );
    if (!allowed)
      throw new Error(`Model missing from allowlist: ${profile.provider}/${profile.model}`);
    if (catalog?.error || !model || model.isSelectable === false)
      throw new Error("Allowed model unavailable.");
    for (const effort of allowed.effortIds) {
      if ((isLunaModel(model.id) || isLunaModel(profile.model)) && effort !== "max")
        throw new Error("Luna only permits max effort.");
      if (!model.thinkingOptions?.some((option) => option.id === effort))
        throw new Error(`Unsupported effort ${effort} for ${profile.model}`);
    }
    for (const option of model.thinkingOptions ?? []) {
      if (!allowed.effortIds.includes(option.id as ModelAllowlist[number]["effortIds"][number]))
        continue;
      candidates.push({
        ...profile,
        id: `c${candidates.length}`,
        profileId: profile.id,
        thinkingOptionId: option.id,
        notes: `${profile.notes ?? "General work"}. Model: ${model.description ?? model.label}. Effort: ${option.description ?? option.label}.`,
      });
    }
  }
  if (
    allowedModels.some(
      (entry) => !profiles.some((p) => p.provider === entry.provider && p.model === entry.model),
    )
  )
    throw new Error("Allowlist model has no selected profile.");
  return candidates;
}

export class DirectRouter {
  private records: DirectRecord[] = [];
  private pending = new Map<string, Promise<DirectResult>>();
  private controller = new AbortController();
  constructor(
    private getApi: () => PaseoApi,
    private judge: Judge,
    private file?: string,
  ) {
    if (!file) return;
    try {
      const stored = JSON.parse(readFileSync(file, "utf8"));
      if (stored.version !== 1 || !Array.isArray(stored.records))
        throw new Error("Invalid direct-routing state.");
      this.records = stored.records;
      for (const record of this.records) {
        if (record.status === "routing" || record.status === "creating") {
          record.status = "interrupted";
          record.error =
            "Plugin restarted during request; inspect agents labeled jev-direct-request before creating another request. No automatic replay.";
        }
      }
      this.save();
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
  }
  private save() {
    if (!this.file) return;
    mkdirSync(path.dirname(this.file), { recursive: true, mode: 0o700 });
    writeFileSync(this.file + ".tmp", JSON.stringify({ version: 1, records: this.records }), {
      mode: 0o600,
    });
    renameSync(this.file + ".tmp", this.file);
  }
  list(workspaceId: string, requestId?: string) {
    return structuredClone(
      this.records.filter(
        (r) => r.workspaceId === workspaceId && (!requestId || r.requestId === requestId),
      ),
    );
  }
  private async workspace(id: string) {
    // The SDK's refresh scans all workspaces. Exact idPrefix avoids that scan.
    const page = await this.getApi().workspaces.list({
      filter: { idPrefix: id },
      page: { limit: 100 },
    });
    const snapshot = page.entries.find((w) => w.id === id);
    if (!snapshot?.workspaceDirectory || snapshot.archivingAt)
      throw new Error("Workspace unavailable or archiving.");
    return {
      handle: this.getApi().workspaces.ref(snapshot),
      cwd: await realpath(snapshot.workspaceDirectory),
    };
  }
  async profiles(workspaceId: string) {
    const { cwd } = await this.workspace(workspaceId);
    const profiles = (await createDriver(this.getApi).profiles(cwd)).filter(
      (p) => p.provider === "codex" || p.provider === "claude",
    );
    const catalogs = new Map(
      await Promise.all(
        [...new Set(profiles.map((p) => p.provider))].map(
          async (provider) =>
            [provider, await this.getApi().providers.listModels(provider, { cwd })] as const,
        ),
      ),
    );
    return profiles.map((profile) => {
      const catalog = catalogs.get(profile.provider);
      const model = catalog?.models?.find(
        (m) => m.id === profile.model || m.aliases?.includes(profile.model),
      );
      const effortIds =
        catalog?.error || !model || model.isSelectable === false
          ? []
          : (model.thinkingOptions ?? [])
              .map((option) => option.id)
              .filter(
                (id) =>
                  effortSchema.safeParse(id).success &&
                  (!(isLunaModel(model.id) || isLunaModel(profile.model)) || id === "max"),
              );
      return { ...profile, effortIds: effortIds as ModelAllowlist[number]["effortIds"] };
    });
  }

  private async candidates(input: DirectInput, cwd: string) {
    const available = await createDriver(this.getApi).profiles(cwd);
    const profiles = input.allowedProfileIds.map((id) => {
      const profile = available.find(
        (p) => p.id === id && ["codex", "claude"].includes(p.provider),
      );
      if (!profile) throw new Error(`Allowed profile unavailable: ${id}`);
      return profile;
    });
    const catalogs = new Map(
      await Promise.all(
        [...new Set(profiles.map((p) => p.provider))].map(
          async (provider) =>
            [provider, await this.getApi().providers.listModels(provider, { cwd })] as const,
        ),
      ),
    );
    const candidates = expandCandidates(profiles, catalogs, input.allowedModels);
    if (!candidates.length) throw new Error("No supported model/effort candidates.");
    return candidates;
  }
  run(raw: DirectInput): Promise<DirectResult> {
    const input = directInput.parse(raw);
    if (new Set(input.allowedProfileIds).size !== input.allowedProfileIds.length)
      throw new Error("Duplicate allowed profile.");
    const fingerprint = createHash("sha256").update(JSON.stringify(input)).digest("hex");
    const old = this.records.find((r) => r.requestId === input.requestId);
    if (old) {
      if (old.fingerprint !== fingerprint)
        throw new Error("Request ID already used for different input.");
      const pending = this.pending.get(input.requestId);
      if (pending) return pending;
      if (old.status === "created" && old.agentId && old.selection && old.routingMs !== undefined)
        return Promise.resolve({
          agentId: old.agentId,
          selection: old.selection,
          routingMs: old.routingMs,
          usage: old.usage,
        });
      throw new Error(
        old.error ?? "Request already started; inspect direct.status. No automatic replay.",
      );
    }
    if (this.controller.signal.aborted) throw new Error("Router stopped.");
    if (this.records.length >= 10000)
      throw new Error(
        "Direct-routing ledger full; archive it while plugin is stopped before submitting new requests.",
      );
    const record: DirectRecord = {
      requestId: input.requestId,
      workspaceId: input.workspaceId,
      fingerprint,
      status: "routing",
      startedAt: Date.now(),
      usage: missingUsage("jev-evaluation", "Evaluation not completed."),
    };
    this.records.push(record);
    this.save();
    const work = this.execute(input, record).finally(() => this.pending.delete(input.requestId));
    this.pending.set(input.requestId, work);
    return work;
  }
  private async execute(input: DirectInput, record: DirectRecord): Promise<DirectResult> {
    try {
      const { cwd } = await this.workspace(input.workspaceId);
      const candidates = await this.candidates(input, cwd);
      const decision = await this.judge(
        "direct",
        { task: input.prompt },
        candidates,
        this.controller.signal,
      );
      record.usage =
        decision.usage ?? missingUsage("jev-evaluation", "Evaluation response omitted usage.");
      const candidate = candidates.find((p) => p.id === decision.profileId);
      if (!candidate) throw new Error("Jev selected an unknown candidate.");
      const { profileId, ...selected } = candidate;
      record.selection = { ...selected, id: profileId };
      record.routingMs = Date.now() - record.startedAt;
      this.save();
      // Revalidate after network evaluation, before creating an agent. Never substitute changed settings.
      const currentWorkspace = await this.workspace(input.workspaceId);
      if (currentWorkspace.cwd !== cwd)
        throw new Error("Workspace directory changed during routing.");
      const current = await this.candidates(input, cwd);
      if (!current.some((p) => JSON.stringify(p) === JSON.stringify(candidate)))
        throw new Error("Candidate settings changed during routing; no agent created.");
      if (this.controller.signal.aborted)
        throw new Error("Routing cancelled before agent creation.");
      record.status = "creating";
      this.save();
      const agent = await currentWorkspace.handle.agents.create({
        config: {
          provider: `${candidate.provider}/${candidate.model}`,
          modeId: candidate.modeId,
          thinkingOptionId: candidate.thinkingOptionId,
          featureValues: candidate.featureValues,
        },
        prompt: input.prompt,
        title: `Jev direct · ${candidate.name} / ${candidate.thinkingOptionId}`,
        env: { PASEO_ORCH_CHILD: "1" },
        labels: { "jev-orchestrator": "direct", "jev-direct-request": input.requestId },
        requestId: input.requestId,
      });
      record.agentId = agent.id;
      record.status = "created";
      record.endedAt = Date.now();
      this.save();
      return {
        agentId: agent.id,
        selection: record.selection,
        routingMs: record.routingMs,
        usage: record.usage,
      };
    } catch (error) {
      if (error && typeof error === "object" && "usage" in error && error.usage)
        record.usage = error.usage as TokenUsage;
      record.error = error instanceof Error ? error.message : "Direct routing failed.";
      record.status = record.status === "creating" ? "interrupted" : "failed";
      if (record.status === "interrupted")
        record.error +=
          " Creation outcome may be unknown; inspect jev-direct-request labels. Do not resubmit with a new ID.";
      record.endedAt = Date.now();
      this.save();
      throw new Error(record.error);
    }
  }
  stop() {
    this.controller.abort();
  }
}
