import test from "node:test";
import assert from "node:assert/strict";
import {
  createSidebarController,
  matchProject,
  sidebarMembership,
  type SidebarSnapshot,
} from "../client/sidebar-state";
import { createSpacesSidebar } from "../client/spaces-sidebar";
import {
  addSpace,
  moveProject,
  preferencesSchema,
  projectKey,
  removeSpace,
  stateSchema,
  withHostState,
} from "../shared/spaces";

test("maps equivalence and placement IDs without project-name guessing", () => {
  const projects = [{ id: "p", name: "Same name", viewKey: "repo:p", workspaces: [] }];
  assert.equal(matchProject("repo:p", projects)?.id, "p");
  assert.equal(matchProject('["host","p"]', projects)?.id, "p");
  assert.equal(matchProject("Same name", projects), undefined);
  const state = moveProject(addSpace(stateSchema.parse({})), projectKey("host", "p"), "space-2");
  assert.equal(sidebarMembership(state, "p"), "space-2");
});
test("revision conflicts preserve old state and report an actionable error", async () => {
  const initial = stateSchema.parse({});
  const client = {
    rpc: async (contract: { name: string }) => {
      if (contract.name === "spaces.catalog") return { projects: [] };
      if (contract.name.endsWith(".read"))
        return { status: "ready", revision: "r1", values: initial };
      return { status: "conflict", error: "Revision conflict" };
    },
  } as unknown as Parameters<typeof createSidebarController>[0];
  const controller = createSidebarController(client);
  await controller.refresh();
  assert.equal(await controller.create(), false);
  assert.equal(controller.get()!.state.spaces.length, 1);
  assert.match(controller.get()!.error, /Revision conflict/);
  controller.stop();
});

test("rename persists through refresh and revision conflict preserves saved name", async () => {
  let saved = addSpace(stateSchema.parse({}));
  let conflict = false;
  const controller = createSidebarController({
    rpc: async (contract: { name: string }, input: { revision: string; values: typeof saved }) => {
      if (contract.name === "spaces.catalog") return { projects: [] };
      if (contract.name.endsWith(".read"))
        return { status: "ready", revision: "r1", values: saved };
      assert.equal(input.revision, "r1");
      if (conflict) return { status: "conflict", error: "Revision conflict" };
      saved = input.values;
      return { status: "saved", revision: "r1", values: saved };
    },
  } as unknown as Parameters<typeof createSidebarController>[0]);
  try {
    await controller.refresh();
    assert.equal(await controller.rename("space-2", "Work"), true);
    await controller.refresh();
    assert.equal(controller.get()!.state.spaces[1].name, "Work");
    conflict = true;
    assert.equal(await controller.rename("space-2", "Other"), false);
    assert.equal(controller.get()!.state.spaces[1].name, "Work");
    assert.match(controller.get()!.error, /Revision conflict/);
  } finally {
    controller.stop();
  }
});

function sidebarWith(host: string | null, error = "") {
  let state = addSpace(stateSchema.parse({}));
  state = moveProject(state, projectKey("view", "repo:p"), "space-2");
  let snapshot: SidebarSnapshot = {
    host,
    state,
    busy: false,
    error,
    projects: [
      { id: "p", name: "P", viewKey: "repo:p", workspaces: [] },
      { id: "q", name: "Q", viewKey: "repo:q", workspaces: [] },
    ],
  };
  const moved: [string, string][] = [];
  let failSave = false;
  const controller = {
    get: () => snapshot,
    getLoadError: () => "",
    notify: () => {},
    subscribe: () => () => {},
    refresh: async () => {},
    create: async () => {
      if (failSave) return false;
      snapshot = { ...snapshot, state: addSpace(snapshot.state) };
      return true;
    },
    rename: async () => true,
    remove: async (id: string) => {
      if (failSave) return false;
      snapshot = { ...snapshot, state: removeSpace(snapshot.state, id) };
      return true;
    },
    move: async () => true,
    moveView: async (key: string, target: string) => {
      moved.push([key, target]);
      snapshot = {
        ...snapshot,
        state: moveProject(snapshot.state, projectKey("view", key), target),
      };
      return true;
    },
    stop: () => {},
  } as unknown as ReturnType<typeof createSidebarController>;
  return {
    sidebar: createSpacesSidebar(controller),
    moved,
    setBusy: (busy: boolean) => {
      snapshot = { ...snapshot, busy };
    },
    failSaves: () => {
      failSave = true;
    },
  };
}
const project = (viewKey: string) => ({ viewKey, name: viewKey, serverIds: [], projectIds: [] });

test("the filter shows only the selected Space's projects for the active host", () => {
  const { sidebar } = sidebarWith("host-a");
  const context = { activeServerId: "host-a" };
  assert.equal(sidebar.isVisible(project("repo:q"), context), true);
  assert.equal(sidebar.isVisible(project("repo:p"), context), false);
  sidebar.select("host-a", "space-2");
  assert.equal(sidebar.isVisible(project("repo:p"), context), true);
  assert.equal(sidebar.isVisible(project("repo:q"), context), false);
  // Projects added outside the app keep the first-Space default.
  assert.equal(sidebar.isVisible(project("repo:new"), context), false);
});

test("errors and a host mismatch show every project", () => {
  assert.equal(
    sidebarWith("host-a", "Save failed").sidebar.isVisible(project("repo:p"), {
      activeServerId: "host-a",
    }),
    true,
  );
  assert.equal(
    sidebarWith("host-a").sidebar.isVisible(project("repo:p"), { activeServerId: "host-b" }),
    true,
  );
});

test("the project menu lists every Space under Move to and moves the project", async () => {
  const { sidebar, moved } = sidebarWith(null);
  const [move] = sidebar.menuItems(project("repo:q"), { activeServerId: null });
  assert.equal(move.title, "Move to");
  const options = move.items ?? [];
  assert.deepEqual(
    options.map((item) => [item.title, item.checked]),
    [
      ["Workspace 1", true],
      ["Workspace 2", false],
    ],
  );
  await options[0].onSelect?.();
  assert.deepEqual(moved, []);
  await options[1].onSelect?.();
  assert.deepEqual(moved, [["repo:q", "space-2"]]);
});

test("a swipe moves to the adjacent Space without wrapping", () => {
  const { sidebar } = sidebarWith("host-a");
  const context = { activeServerId: "host-a" };
  sidebar.onSwipe(-1, context);
  assert.equal(sidebar.selectedSpace("host-a"), "space-1");
  sidebar.onSwipe(1, context);
  assert.equal(sidebar.selectedSpace("host-a"), "space-2");
  sidebar.onSwipe(1, context);
  assert.equal(sidebar.selectedSpace("host-a"), "space-2");
  // Another host's swipe does not change this host's Space.
  sidebar.onSwipe(-1, { activeServerId: "host-b" });
  assert.equal(sidebar.selectedSpace("host-a"), "space-2");
});

test("a swipe does nothing while a save is in flight", () => {
  const { sidebar, setBusy } = sidebarWith("host-a");
  const context = { activeServerId: "host-a" };
  setBusy(true);
  sidebar.onSwipe(1, context);
  assert.equal(sidebar.selectedSpace("host-a"), "space-1");
  setBusy(false);
  sidebar.onSwipe(1, context);
  assert.equal(sidebar.selectedSpace("host-a"), "space-2");
});

test("creating a Space selects the new one, and a failed create keeps the selection", async () => {
  const { sidebar, failSaves } = sidebarWith("host-a");
  assert.equal(await sidebar.create("host-a"), true);
  assert.equal(sidebar.selectedSpace("host-a"), "space-3");
  failSaves();
  assert.equal(await sidebar.create("host-a"), false);
  assert.equal(sidebar.selectedSpace("host-a"), "space-3");
});

test("removing the selected Space selects its removal target", async () => {
  const { sidebar } = sidebarWith("host-a");
  await sidebar.create("host-a");
  await sidebar.create("host-a");
  assert.equal(sidebar.selectedSpace("host-a"), "space-4");
  // The preceding Space is selected, not Space 1.
  assert.equal(await sidebar.remove("host-a", "space-4"), true);
  assert.equal(sidebar.selectedSpace("host-a"), "space-3");
  // Removing the first Space selects the next one.
  sidebar.select("host-a", "space-1");
  assert.equal(await sidebar.remove("host-a", "space-1"), true);
  assert.equal(sidebar.selectedSpace("host-a"), "space-2");
});

test("removing another Space or failing to remove keeps the selection", async () => {
  const { sidebar, failSaves } = sidebarWith("host-a");
  await sidebar.create("host-a");
  sidebar.select("host-a", "space-2");
  assert.equal(await sidebar.remove("host-a", "space-3"), true);
  assert.equal(sidebar.selectedSpace("host-a"), "space-2");
  failSaves();
  assert.equal(await sidebar.remove("host-a", "space-2"), false);
  assert.equal(sidebar.selectedSpace("host-a"), "space-2");
});

test("changing Space reports the slide direction to the host", () => {
  const { sidebar } = sidebarWith("host-a");
  const changes: unknown[] = [];
  const stop = sidebar.subscribe((change) => changes.push(change));
  sidebar.select("host-a", "space-2");
  sidebar.select("host-a", "space-2");
  sidebar.onSwipe(-1, { activeServerId: "host-a" });
  stop();
  assert.deepEqual(changes, [{ direction: 1 }, undefined, { direction: -1 }]);
});

test("the heading title is the selected Space, and errors or another host keep Workspaces", () => {
  const { sidebar } = sidebarWith("host-a");
  assert.equal(sidebar.getTitle({ activeServerId: "host-a" }), "Workspace 1");
  sidebar.select("host-a", "space-2");
  assert.equal(sidebar.getTitle({ activeServerId: "host-a" }), "Workspace 2");
  assert.equal(sidebar.getTitle({ activeServerId: "host-b" }), null);
  assert.equal(
    sidebarWith("host-a", "Save failed").sidebar.getTitle({ activeServerId: "host-a" }),
    null,
  );
});

test("Move projects lists each project's Space and moves a project by its view key", async () => {
  const { sidebar, moved } = sidebarWith("host-a");
  assert.deepEqual(sidebar.projects("host-a"), [
    { id: "p", name: "P", spaceId: "space-2" },
    { id: "q", name: "Q", spaceId: "space-1" },
  ]);
  assert.deepEqual(sidebar.projects("host-b"), []);
  assert.equal(await sidebar.moveProject("host-a", "q", "space-2"), true);
  assert.deepEqual(moved, [["repo:q", "space-2"]]);
  assert.equal(await sidebar.moveProject("host-a", "missing", "space-2"), false);
});

function additionFixture(initialHost: string | null) {
  const state = stateSchema.parse({
    spaces: [
      { id: "space-1", name: "Silentium" },
      { id: "space-2", name: "Misc" },
    ],
  });
  let prefs = withHostState(preferencesSchema.parse({}), initialHost, state);
  let host = initialHost;
  let conflict = false;
  let readGate: Promise<void> | null = null;
  const controller = createSidebarController(
    {
      rpc: async (contract: { name: string }, input: { values: typeof prefs }) => {
        if (contract.name === "spaces.catalog") return { projects: [] };
        if (contract.name.endsWith(".read")) {
          await readGate;
          return { status: "ready", revision: "r1", values: prefs };
        }
        if (conflict) return { status: "conflict", error: "Revision conflict" };
        prefs = preferencesSchema.parse(input.values);
        return { status: "saved", revision: "r1", values: prefs };
      },
    } as unknown as Parameters<typeof createSidebarController>[0],
    () => host,
  );
  return {
    controller,
    sidebar: createSpacesSidebar(controller),
    prefs: () => prefs,
    setHost: (next: string | null) => {
      host = next;
    },
    failSave: () => {
      conflict = true;
    },
    holdRead: (gate: Promise<void>) => {
      readGate = gate;
    },
  };
}

for (const host of ["host-a", null]) {
  test(`Add Project persists Misc membership in ${host ?? "All hosts"}`, async () => {
    const { controller, sidebar, prefs } = additionFixture(host);
    try {
      await controller.refresh();
      sidebar.select(host, "space-2");
      const added = {
        viewKey: "directory:.claude",
        name: ".claude",
        serverIds: ["host-a"],
        projectIds: ["claude"],
      };
      await sidebar.onProjectAdded(added, { activeServerId: host });
      await controller.refresh();
      assert.equal(sidebar.isVisible(added, { activeServerId: host }), true);
      assert.equal(sidebarMembership(controller.get()!.state, "claude"), "space-2");
      sidebar.select(host, "space-1");
      assert.equal(sidebar.isVisible(added, { activeServerId: host }), false);
      if (host) assert.deepEqual(prefs().members, {});
      else assert.deepEqual(prefs().hosts, {});
    } finally {
      controller.stop();
    }
  });
}

test("Add Project preserves an existing assignment and reports save conflicts", async () => {
  const { controller, sidebar, failSave } = additionFixture("host-a");
  const added = {
    viewKey: '["host-a","claude"]',
    name: ".claude",
    serverIds: ["host-a"],
    projectIds: ["claude"],
  };
  try {
    await controller.refresh();
    sidebar.select("host-a", "space-2");
    await sidebar.onProjectAdded(added, { activeServerId: "host-a" });
    sidebar.select("host-a", "space-1");
    await sidebar.onProjectAdded(added, { activeServerId: "host-a" });
    assert.equal(sidebarMembership(controller.get()!.state, "claude"), "space-2");
    failSave();
    await assert.rejects(
      sidebar.onProjectAdded(
        { ...added, viewKey: "new", projectIds: ["new"] },
        { activeServerId: "host-a" },
      ),
      /Revision conflict/,
    );
    assert.equal(
      Object.hasOwn(controller.get()!.state.members, projectKey("sidebar", "new")),
      false,
    );
  } finally {
    controller.stop();
  }
});

test("Add Project waits for an in-flight catalog refresh and saves the captured host", async () => {
  const { controller, sidebar, setHost, holdRead, prefs } = additionFixture("host-a");
  try {
    await controller.refresh();
    sidebar.select("host-a", "space-2");
    let release!: () => void;
    holdRead(
      new Promise<void>((resolve) => {
        release = resolve;
      }),
    );
    const refreshing = controller.refresh();
    const adding = sidebar.onProjectAdded(
      { viewKey: "new", name: "New", serverIds: ["host-a"], projectIds: ["new"] },
      { activeServerId: "host-a" },
    );
    setHost("host-b");
    release();
    await Promise.all([refreshing, adding]);
    assert.equal(prefs().hosts["host-a"].members[projectKey("sidebar", "new")], "space-2");
    assert.equal(Object.hasOwn(prefs().hosts, "host-b"), false);
  } finally {
    controller.stop();
  }
});
