import { describe, expect, test } from "vitest";
import { DirectorySyncService } from "./index.js";

// The constructor infers its parameter as the randomUUID() template type.
const GENERATION = "11111111-1111-4111-8111-111111111111";
const NEW_GENERATION = "22222222-2222-4222-8222-222222222222";

function project(projectId: string, name: string) {
  return {
    projectId,
    projectDisplayName: name,
    projectRootPath: `/${projectId}`,
    projectKind: "git" as const,
  };
}

describe("DirectorySyncService", () => {
  test("owns sequencing and returns wire-ready latest project changes", () => {
    const service = new DirectorySyncService(GENERATION);
    expect(service.synchronizeProjects([project("one", "One")], {}).sync).toMatchObject({
      mode: "snapshot",
      headSeq: 1,
    });

    service.sequenceProjectUpdate({ kind: "upsert", project: project("one", "Renamed") }, true);
    service.sequenceProjectUpdate({ kind: "upsert", project: project("one", "Latest") }, true);
    expect(
      service.synchronizeProjects([project("one", "Latest")], {
        generation: GENERATION,
        afterSeq: 1,
      }),
    ).toEqual({
      projects: [{ ...project("one", "Latest"), syncSeq: 3 }],
      sync: { generation: GENERATION, mode: "changes", headSeq: 3, removals: [] },
    });
  });

  test("sequences a hard project removal once", () => {
    const service = new DirectorySyncService(GENERATION);
    service.synchronizeProjects([project("one", "One")], {});
    const update = { kind: "remove" as const, projectId: "one" };
    expect(service.sequenceProjectUpdate(update, true)).toEqual({
      ...update,
      generation: GENERATION,
      seq: 2,
    });
    expect(service.sequenceProjectUpdate(update, true)).toEqual({
      ...update,
      generation: GENERATION,
      seq: 2,
    });
    expect(
      service.synchronizeProjects([], { generation: GENERATION, afterSeq: 1 }).sync?.removals,
    ).toEqual([{ id: "one", seq: 2 }]);
  });

  test("keeps one daemon-wide version for repeated observations", () => {
    const service = new DirectorySyncService(GENERATION);
    const update = { kind: "upsert" as const, project: project("one", "One") };
    const sequenced = { ...update, generation: GENERATION, seq: 1 };
    expect(service.sequenceProjectUpdate(update, true)).toEqual(sequenced);
    expect(service.sequenceProjectUpdate(update, true)).toEqual(sequenced);
  });

  test("sequences only the project whose effective icon changed", () => {
    const service = new DirectorySyncService(GENERATION);
    const one = { ...project("one", "One"), projectIconRevision: "icon-a" };
    const two = { ...project("two", "Two"), projectIconRevision: "icon-a" };
    service.synchronizeProjects([one, two], {});
    const changed = { ...one, projectIconRevision: "icon-b" };
    service.sequenceProjectUpdate({ kind: "upsert", project: changed }, true);

    expect(
      service.synchronizeProjects([changed, two], {
        generation: GENERATION,
        afterSeq: 2,
      }),
    ).toEqual({
      projects: [{ ...changed, syncSeq: 3 }],
      sync: { generation: GENERATION, mode: "changes", headSeq: 3, removals: [] },
    });
  });

  test("falls back to a snapshot when the daemon generation changes", () => {
    const service = new DirectorySyncService(NEW_GENERATION);
    const read = service.synchronizeProjects([project("one", "One")], {
      generation: "old-generation",
      afterSeq: 10,
    });
    expect(read.sync).toMatchObject({
      mode: "snapshot",
      reason: "generation_changed",
      headSeq: 1,
    });
  });
});
