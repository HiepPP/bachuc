import path from "node:path";
import { realpath, lstat } from "node:fs/promises";
import { createHash } from "node:crypto";
import type { Job, Metric, Profile } from "./types";
import type { Task } from "../shared/contracts";

export function ownedPath(value: string): string {
  const normalized = path.posix.normalize(value);
  if (
    path.isAbsolute(value) ||
    value.includes("\\") ||
    normalized === ".." ||
    normalized.startsWith("../") ||
    value.includes("\0") ||
    /[*?[\]]/.test(value)
  )
    throw new Error("Ownership must use workspace-relative paths, not globs.");
  return normalized.replace(/\/$/, "");
}
export async function validatePaths(cwd: string, files: string[]) {
  const root = await realpath(cwd);
  for (const file of files) {
    let current = path.resolve(root, ownedPath(file));
    while (current !== root) {
      try {
        await lstat(current);
        const resolved = await realpath(current);
        if (resolved !== root && !resolved.startsWith(root + path.sep))
          throw new Error("Owned path escapes workspace through a symlink.");
        if (resolved !== current)
          throw new Error("Use canonical ownership paths, not symlink aliases.");
        break;
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
        current = path.dirname(current);
      }
    }
  }
}
export function overlap(a: string, b: string) {
  return a === "." || b === "." || a === b || a.startsWith(b + "/") || b.startsWith(a + "/");
}
export function conflict(a: Job, b: Job) {
  return (
    a.cwd === b.cwd &&
    (a.task.files.some((x) => b.task.files.some((y) => overlap(x, y))) ||
      a.task.resources.some((x) => b.task.resources.includes(x)))
  );
}
export function validateGraph(tasks: Task[], existing: Job[]) {
  const all = new Map(existing.map((j) => [j.task.id, j.task]));
  for (const t of tasks) all.set(t.id, t);
  const active = new Set<string>(),
    seen = new Set<string>();
  function visit(id: string) {
    if (active.has(id)) throw new Error("Dependency cycle.");
    if (seen.has(id)) return;
    const t = all.get(id);
    if (!t) throw new Error("Unknown dependency: " + id);
    active.add(id);
    for (const dep of t.dependsOn) visit(dep);
    active.delete(id);
    seen.add(id);
  }
  for (const t of tasks) visit(t.id);
}
export const profileVersion = (p: Profile) =>
  createHash("sha256").update(JSON.stringify(p)).digest("hex").slice(0, 16);
export function rankHistory(profiles: Profile[], metrics: Metric[], category: string) {
  return profiles.map((profile) => {
    const samples = metrics.filter(
      (m) =>
        m.profileId === profile.id &&
        m.profileVersion === profileVersion(profile) &&
        m.category === category &&
        m.phase === "implementation",
    );
    return {
      profileId: profile.id,
      samples: samples.length,
      passRate: samples.length ? samples.filter((s) => s.verified).length / samples.length : null,
      meanMs: samples.length
        ? samples.reduce((n, s) => n + s.durationMs, 0) / samples.length
        : null,
    };
  });
}
export function chooseMeasured(
  selected: Profile,
  candidates: Profile[],
  probabilities: Record<string, number> | undefined,
  metrics: Metric[],
  category: string,
): Profile {
  if (!probabilities) return selected;
  // Only comparable candidates with real check history may displace the evaluator's first choice.
  const plausible = rankHistory(candidates, metrics, category).filter(
    (s) =>
      s.samples >= 3 &&
      s.passRate! >= 0.8 &&
      (probabilities[s.profileId] ?? 0) >= (probabilities[selected.id] ?? 1) - 0.1,
  );
  plausible.sort((a, b) => b.passRate! - a.passRate! || a.meanMs! - b.meanMs!);
  return candidates.find((p) => p.id === plausible[0]?.profileId) ?? selected;
}
