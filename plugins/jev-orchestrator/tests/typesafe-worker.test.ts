import assert from "node:assert/strict";
import test from "node:test";
import { spawn } from "node:child_process";
import { mkdtemp, writeFile, unlink, rmdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

test("real worker uses direct Typesafe, validates responses, and never retries or falls back", async () => {
  const folder = await mkdtemp(path.join(tmpdir(), "jev-typesafe-test-"));
  const credential = path.join(folder, "typesafe-ai.json");
  const root = path.resolve(import.meta.dirname, "..");
  const key = "fake-typesafe-key";
  await writeFile(credential, JSON.stringify({ apiKey: key }));
  try {
    for (const scenario of ["success", "401", "429", "invalid", "missing"]) {
      await writeFile(credential, JSON.stringify(scenario === "missing" ? {} : { apiKey: key }));
      const mock = `
        let calls = 0;
        globalThis.fetch = async (url, init) => {
          if (++calls > 1) throw new Error('unexpected retry');
          if (String(url) !== 'https://api.typesafe.ai/v1/systemone') throw new Error('unexpected endpoint');
          if (new Headers(init.headers).get('authorization') !== 'Bearer ${key}') throw new Error('wrong key');
          const body = JSON.parse(init.body);
          if (body.model !== 'jev-latest' || body.questions.profile.type !== 'choice') throw new Error('wrong request');
          const status = ${scenario === "401" ? 401 : scenario === "429" ? 429 : 200};
          const response = status !== 200 ? {error: {message: 'private-upstream-body'}} : {
            model: 'jev-latest', answers: {profile: {type: 'choice', choice: '${scenario === "invalid" ? "unknown" : "c0"}', probabilities: {c0: 1}, confidence: 1}},
            usage: {input_tokens: 20, output_tokens: 4}
          };
          return new Response(JSON.stringify(response), {status, headers: {'content-type': 'application/json'}});
        };
        process.on('exit', () => process.stderr.write('calls=' + calls));
      `;
      const child = spawn(
        process.execPath,
        [
          "--import",
          path.join(root, "node_modules/tsx/dist/loader.mjs"),
          "--import",
          `data:text/javascript,${encodeURIComponent(mock)}`,
          path.join(root, "server/jev-worker.ts"),
        ],
        { stdio: ["pipe", "pipe", "pipe"] },
      );
      let stdout = "",
        stderr = "";
      child.stdout.on("data", (chunk) => {
        stdout += chunk;
      });
      child.stderr.on("data", (chunk) => {
        stderr += chunk;
      });
      const closed = new Promise<number | null>((resolve, reject) => {
        child.once("error", reject);
        child.once("close", resolve);
      });
      child.stdin.end(
        JSON.stringify({
          configFile: path.join(folder, "config.json"),
          phase: "direct",
          state: { task: "fixture" },
          profiles: [{ id: "c0", name: "fixture", provider: "codex", model: "fixture" }],
        }),
      );
      const code = await closed;
      const result = JSON.parse(stdout);
      assert.match(stderr, new RegExp(`calls=${scenario === "missing" ? 0 : 1}$`));
      assert.doesNotMatch(stdout, /fake-typesafe-key|private-upstream-body/);
      if (scenario === "success") {
        assert.equal(code, 0);
        assert.equal(result.profileId, "c0");
        assert.equal(result.usage.totalTokens, 24);
      } else {
        assert.equal(code, 1);
        assert.equal(
          result.diagnostics.workerCode,
          scenario === "missing"
            ? "JEV_CREDENTIAL_MISSING"
            : scenario === "invalid"
              ? "JEV_TYPESAFE_FAILED"
              : "JEV_TYPESAFE_HTTP",
        );
        if (scenario === "401" || scenario === "429")
          assert.equal(result.diagnostics.httpStatus, Number(scenario));
      }
    }
  } finally {
    await unlink(credential);
    await rmdir(folder);
  }
});
