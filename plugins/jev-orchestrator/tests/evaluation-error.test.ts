import assert from "node:assert/strict";
import test from "node:test";
import { mkdir, mkdtemp, writeFile, unlink, rmdir } from "node:fs/promises";
import path from "node:path";
import { tmpdir } from "node:os";
import {
  EvaluationError,
  safeEvaluationDiagnostics,
  gatewayEvaluationError,
} from "../server/evaluation-error";
import { createJudge } from "../server/jev";

test("diagnostics allowlist drops arbitrary fields and invalid numeric values", () => {
  assert.equal(safeEvaluationDiagnostics({ workerCode: "secret-token" }), undefined);
  assert.deepEqual(
    safeEvaluationDiagnostics({
      workerCode: "JEV_GATEWAY_HTTP",
      httpStatus: 429,
      processExitCode: 1,
      message: "secret",
      stack: "private",
    }),
    { workerCode: "JEV_GATEWAY_HTTP", httpStatus: 429, processExitCode: 1 },
  );
  assert.deepEqual(
    safeEvaluationDiagnostics({
      workerCode: "JEV_GATEWAY_FAILED",
      httpStatus: "401",
      processExitCode: -1,
    }),
    { workerCode: "JEV_GATEWAY_FAILED" },
  );
});

test("worker wrapper preserves safe details and real exit code without invoking Gateway", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "jev-worker-fixture-"));
  const loaderDir = path.join(root, "node_modules/tsx/dist");
  const serverDir = path.join(root, "server");
  await mkdir(loaderDir, { recursive: true });
  await mkdir(serverDir);
  const loader = path.join(loaderDir, "loader.mjs"),
    worker = path.join(serverDir, "jev-worker.ts");
  await writeFile(loader, "");
  try {
    for (const fixture of [
      {
        body: JSON.stringify({
          error: "secret raw body",
          diagnostics: {
            workerCode: "JEV_GATEWAY_HTTP",
            httpStatus: 429,
            processExitCode: 99,
            secret: "private",
          },
        }),
        exit: 1,
        expected: "JEV_GATEWAY_HTTP",
        http: 429,
      },
      {
        body: "secret invalid output",
        exit: 7,
        expected: "JEV_WORKER_EXIT_FAILED",
        http: undefined,
      },
      {
        body: "secret invalid output",
        exit: 0,
        expected: "JEV_WORKER_OUTPUT_INVALID",
        http: undefined,
      },
      {
        body: JSON.stringify({ profileId: "unknown" }),
        exit: 0,
        expected: "JEV_RESPONSE_INVALID",
        http: undefined,
      },
    ]) {
      await writeFile(
        worker,
        `process.stdin.resume(); process.stdin.on('end', () => { process.stdout.write(${JSON.stringify(fixture.body)}); process.exitCode = ${fixture.exit}; });`,
      );
      const judge = createJudge("unused-fixture", root, 0);
      await assert.rejects(
        judge(
          "direct",
          {},
          [{ id: "c0", name: "fixture", provider: "codex", model: "fixture" }],
          new AbortController().signal,
        ),
        (error: EvaluationError) => {
          assert.equal(error.diagnostics.workerCode, fixture.expected);
          assert.equal(error.diagnostics.httpStatus, fixture.http);
          assert.equal(error.diagnostics.processExitCode, fixture.exit);
          assert.doesNotMatch(JSON.stringify(error), /secret|private/);
          assert.doesNotMatch(error.message, /secret|private/);
          return true;
        },
      );
    }
  } finally {
    await unlink(worker);
    await unlink(loader);
    await rmdir(serverDir);
    await rmdir(loaderDir);
    await rmdir(path.dirname(loaderDir));
    await rmdir(path.join(root, "node_modules"));
    await rmdir(root);
  }
});

test("Gateway free-tier restriction is explicit and redacts raw details", () => {
  const failure = gatewayEvaluationError({
    statusCode: 403,
    message: "Gateway failed",
    cause: {
      responseBody: JSON.stringify({
        error: {
          message:
            "RestrictedModelsError: Free tier users do not have access to this model. secret-key private-account",
        },
      }),
    },
  });
  assert.equal(failure.diagnostics.workerCode, "JEV_MODEL_ACCESS_DENIED");
  assert.equal(failure.diagnostics.httpStatus, 403);
  assert.match(failure.message, /account tier/);
  assert.doesNotMatch(JSON.stringify(failure) + failure.message, /secret-key|private-account/);
  assert.equal(
    gatewayEvaluationError({ statusCode: 403, message: "Other policy denied" }).diagnostics
      .workerCode,
    "JEV_GATEWAY_HTTP",
  );
  assert.equal(
    gatewayEvaluationError({ statusCode: 429, message: "RestrictedModelsError" }).diagnostics
      .workerCode,
    "JEV_GATEWAY_HTTP",
  );
});
