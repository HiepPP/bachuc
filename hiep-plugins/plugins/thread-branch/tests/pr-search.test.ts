import assert from "node:assert/strict";
import test from "node:test";
import {
  createPrSearch,
  cutBody,
  githubRepo,
  MAX_BODY,
  MAX_LOG,
  tailLog,
  type Runner,
} from "../server/pr-search";

const ok = (stdout: string) => ({ code: 0, stdout, stderr: "", enoent: false });
const tick = () => new Promise((resolve) => setTimeout(resolve, 5));

function pr(number: number, overrides: Record<string, unknown> = {}) {
  return {
    number,
    title: `Change ${number}`,
    body: `Body ${number}`,
    headRefName: `feat/change-${number}`,
    author: { login: "dev" },
    url: `https://github.com/o/r/pull/${number}`,
    updatedAt: `2026-09-${String(10 + (number % 20)).padStart(2, "0")}T00:00:00Z`,
    statusCheckRollup: [],
    ...overrides,
  };
}

function fake(options: {
  remotes: Record<string, string>;
  prs?: Record<string, unknown[]>;
  logs?: Record<string, string | { code: number; stdout: string; stderr: string }>;
  missing?: boolean;
  delay?: number;
}) {
  const calls: string[][] = [];
  let active = 0;
  let maxActive = 0;
  const run: Runner = async (file, args, cwd) => {
    calls.push([file, ...args]);
    if (file === "git") {
      const remote = options.remotes[cwd];
      return remote
        ? ok(`${remote}\n`)
        : { code: 2, stdout: "", stderr: "no origin", enoent: false };
    }
    if (options.missing) return { code: null, stdout: "", stderr: "", enoent: true };
    active++;
    maxActive = Math.max(maxActive, active);
    await new Promise((resolve) => setTimeout(resolve, options.delay ?? 0));
    active--;
    if (args[0] === "run") {
      const log = options.logs?.[args[2]] ?? "";
      return typeof log === "string" ? ok(log) : { ...log, enoent: false };
    }
    return ok(JSON.stringify(options.prs?.[args[args.indexOf("--repo") + 1]] ?? []));
  };
  return {
    run,
    calls,
    gh: () => calls.filter((call) => call[0] === "gh"),
    maxActive: () => maxActive,
  };
}

const dirs =
  (...list: string[]) =>
  async () =>
    list;

test("keeps GitHub remotes only and removes duplicate repos", async () => {
  assert.equal(githubRepo("git@github.com:o/r.git"), "o/r");
  assert.equal(githubRepo("https://github.com/o/r"), "o/r");
  assert.equal(githubRepo("https://gitlab.com/g/r.git"), null);
  assert.equal(githubRepo("/local/path.git"), null);
  const runner = fake({
    remotes: {
      "/a": "git@github.com:o/r.git",
      "/b": "https://github.com/O/R",
      "/c": "https://gitlab.com/g/r.git",
      "/d": "https://github.com/o/other.git",
    },
    prs: { "o/r": [pr(1)], "o/other": [pr(2, { url: "https://github.com/o/other/pull/2" })] },
  });
  const search = createPrSearch({ run: runner.run });
  const { items } = await search.search("", dirs("/a", "/b", "/c", "/d", "/e"));
  assert.deepEqual(
    runner.gh().map((call) => call[call.indexOf("--repo") + 1]),
    ["o/r", "o/other"],
  );
  assert.deepEqual(items.map((item) => item.id).sort(), ["o/other#2", "o/r#1"]);
});

test("matches repo, number, title, and branch; empty query lists newest first", async () => {
  const runner = fake({
    remotes: { "/a": "git@github.com:o/r.git" },
    prs: {
      "o/r": [
        pr(3, { title: "Fix login", updatedAt: "2026-09-01T00:00:00Z" }),
        pr(4, { headRefName: "chore/deps", updatedAt: "2026-09-03T00:00:00Z" }),
        pr(5, { updatedAt: "2026-09-02T00:00:00Z" }),
      ],
    },
  });
  const search = createPrSearch({ run: runner.run });
  const ids = async (query: string) =>
    (await search.search(query, dirs("/a"))).items.map((item) => item.identifier);
  assert.deepEqual(await ids(""), ["#4", "#5", "#3"]);
  assert.deepEqual(await ids("login"), ["#3"]);
  assert.deepEqual(await ids("#5"), ["#5"]);
  assert.deepEqual(await ids("chore/deps"), ["#4"]);
  assert.deepEqual(await ids("o/r fix"), ["#3"]);
  assert.deepEqual(await ids("nothing"), []);
});

test("returns at most 20 items across repos", async () => {
  const many = (repo: string, start: number) =>
    Array.from({ length: 15 }, (_, index) =>
      pr(start + index, { url: `https://github.com/${repo}/pull/${start + index}` }),
    );
  const runner = fake({
    remotes: { "/a": "git@github.com:o/a.git", "/b": "git@github.com:o/b.git" },
    prs: { "o/a": many("o/a", 1), "o/b": many("o/b", 100) },
  });
  const { items } = await createPrSearch({ run: runner.run }).search("", dirs("/a", "/b"));
  assert.equal(items.length, 20);
});

test("cuts the PR body and the CI log", async () => {
  assert.equal(cutBody("short"), "short");
  assert.ok(cutBody("x".repeat(MAX_BODY + 50)).startsWith("x".repeat(MAX_BODY)));
  assert.ok(cutBody("x".repeat(MAX_BODY + 50)).length < MAX_BODY + 20);
  const lines = Array.from({ length: 300 }, (_, index) => `line ${index}`).join("\n");
  const tail = tailLog(lines).split("\n");
  assert.equal(tail.length, 200);
  assert.equal(tail.at(-1), "line 299");
  assert.equal(tailLog("y".repeat(20_000)).length, MAX_LOG);

  const runner = fake({
    remotes: { "/a": "git@github.com:o/r.git" },
    prs: { "o/r": [pr(1, { body: "Q".repeat(MAX_BODY + 500) })] },
  });
  const { items } = await createPrSearch({ run: runner.run }).search("", dirs("/a"));
  assert.match(items[0].text, /Repository: o\/r/);
  assert.match(items[0].text, /Branch: feat\/change-1/);
  assert.match(items[0].text, /Author: dev/);
  assert.match(items[0].text, /URL: https:\/\/github.com\/o\/r\/pull\/1/);
  assert.match(items[0].text, /Checks: no checks/);
  assert.equal(items[0].text.match(/Q/g)?.length, MAX_BODY);
});

test("caches repos and PR lists for 5 minutes", async () => {
  let now = 0;
  const runner = fake({ remotes: { "/a": "git@github.com:o/r.git" }, prs: { "o/r": [pr(1)] } });
  const search = createPrSearch({ run: runner.run, now: () => now });
  await search.search("", dirs("/a"));
  now += 4 * 60_000;
  await search.search("change", dirs("/a"));
  assert.equal(runner.calls.length, 2);
  now += 60_000;
  await search.search("", dirs("/a"));
  assert.equal(runner.calls.length, 4);
});

test("runs at most 3 gh calls at once", async () => {
  const remotes: Record<string, string> = {};
  for (let index = 0; index < 7; index++) remotes[`/w${index}`] = `git@github.com:o/r${index}.git`;
  const runner = fake({ remotes, delay: 10 });
  await createPrSearch({ run: runner.run }).search("", dirs(...Object.keys(remotes)));
  assert.equal(runner.gh().length, 7);
  assert.equal(runner.maxActive(), 3);
});

test("offers a CI failure item after the failed log is fetched in the background", async () => {
  const failing = pr(7, {
    statusCheckRollup: [
      {
        __typename: "CheckRun",
        name: "build",
        conclusion: "FAILURE",
        detailsUrl: "https://github.com/o/r/actions/runs/99/job/1",
      },
      { __typename: "CheckRun", name: "lint", conclusion: "SUCCESS" },
    ],
  });
  const log = Array.from({ length: 250 }, (_, index) => `err ${index}`).join("\n");
  const runner = fake({
    remotes: { "/a": "git@github.com:o/r.git" },
    prs: { "o/r": [failing] },
    logs: { "99": log },
  });
  const search = createPrSearch({ run: runner.run });
  const first = await search.search("", dirs("/a"));
  assert.equal(first.items.length, 1);
  assert.match(first.items[0].subtitle ?? "", /1 passed, 1 failed \(build\)/);
  await tick();
  const second = await search.search("", dirs("/a"));
  assert.deepEqual(
    second.items.map((item) => item.title),
    ["Change 7", "CI failure: #7 build"],
  );
  const ci = second.items[1];
  assert.equal(ci.url, "https://github.com/o/r/actions/runs/99");
  assert.match(ci.text, /err 249/);
  assert.doesNotMatch(ci.text, /err 49\n/);
  assert.deepEqual(
    runner.gh().filter((call) => call[1] === "run"),
    [["gh", "run", "view", "99", "--repo", "o/r", "--log-failed"]],
  );
});

for (const stream of ["stdout", "stderr"] as const) {
  test(`treats a running workflow as not ready (notice on ${stream})`, async () => {
    let now = 0;
    const lines: string[] = [];
    const notice = "run 99 is still in progress; logs will be available when it is complete";
    const logs: Record<string, string | { code: number; stdout: string; stderr: string }> = {
      "99": {
        code: 0,
        stdout: stream === "stdout" ? notice : "",
        stderr: stream === "stderr" ? notice : "",
      },
    };
    const runner = fake({
      remotes: { "/a": "git@github.com:o/r.git" },
      prs: { "o/r": [failingPr()] },
      logs,
    });
    const search = createPrSearch({
      run: runner.run,
      now: () => now,
      log: (line) => lines.push(line),
    });
    const runs = () => runner.gh().filter((call) => call[1] === "run").length;
    await search.search("", dirs("/a"));
    await tick();
    assert.deepEqual(
      (await search.search("", dirs("/a"))).items.map((item) => item.title),
      ["Change 7"],
    );
    assert.equal(runs(), 1);
    now += 59_000;
    await search.search("", dirs("/a"));
    assert.equal(runs(), 1);
    logs["99"] = "err 1\nerr 2";
    now += 2_000;
    await search.search("", dirs("/a"));
    await tick();
    assert.equal(runs(), 2);
    assert.deepEqual(
      (await search.search("", dirs("/a"))).items.map((item) => item.title),
      ["Change 7", "CI failure: #7 build"],
    );
    assert.deepEqual(lines, []);
  });
}

test("still logs a real CI log failure once and keeps it for 5 minutes", async () => {
  let now = 0;
  const lines: string[] = [];
  const runner = fake({
    remotes: { "/a": "git@github.com:o/r.git" },
    prs: { "o/r": [failingPr()] },
    logs: { "99": { code: 1, stdout: "", stderr: "HTTP 404: Not Found" } },
  });
  const search = createPrSearch({
    run: runner.run,
    now: () => now,
    log: (line) => lines.push(line),
  });
  const runs = () => runner.gh().filter((call) => call[1] === "run").length;
  await search.search("", dirs("/a"));
  await tick();
  now += 2 * 60_000;
  await search.search("", dirs("/a"));
  assert.equal(runs(), 1);
  assert.equal(lines.length, 1);
  assert.match(lines[0], /gh run view 99 failed/);
});

function failingPr() {
  return pr(7, {
    statusCheckRollup: [
      {
        name: "build",
        conclusion: "FAILURE",
        detailsUrl: "https://github.com/o/r/actions/runs/99/job/1",
      },
    ],
  });
}

test("returns no items and logs one line when gh is missing", async () => {
  let now = 0;
  const lines: string[] = [];
  const runner = fake({ remotes: { "/a": "git@github.com:o/r.git" }, missing: true });
  const search = createPrSearch({
    run: runner.run,
    now: () => now,
    log: (line) => lines.push(line),
  });
  assert.deepEqual(await search.search("", dirs("/a")), { items: [] });
  now += 10 * 60_000;
  assert.deepEqual(await search.search("", dirs("/a")), { items: [] });
  assert.equal(runner.gh().length, 2);
  assert.equal(lines.length, 1);
  assert.match(lines[0], /gh not found/);
});
