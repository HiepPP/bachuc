import assert from "node:assert/strict";
import test from "node:test";
import { nativeBindingError, requireNativeBinding } from "../server/native-binding";
const env = { PASEO_ORCH_URL: "http://127.0.0.1:1234/mcp", PASEO_ORCH_TOKEN: "secret-fixture" };
test("binding errors identify missing fields without exposing values", () => {
  assert.match(nativeBindingError({})!, /PASEO_ORCH_URL, PASEO_ORCH_TOKEN/);
  assert.match(
    nativeBindingError({ PASEO_ORCH_URL: env.PASEO_ORCH_URL })!,
    /missing bridge binding \(PASEO_ORCH_TOKEN\)/,
  );
  const error = nativeBindingError({ ...env, PASEO_ORCH_URL: "https://secret.example/mcp" })!;
  assert.match(error, /invalid bridge URL/);
  assert.doesNotMatch(error, /secret/);
  assert.equal(nativeBindingError(env), undefined);
});
test("native preparation waits for ready bridge and valid lease", async () => {
  let resolve!: (url: string) => void;
  let bound = false,
    prepared = false;
  const bridge = {
    ready: new Promise<string>((r) => {
      resolve = r;
    }),
    bind(token: string, id: string, cwd: string) {
      assert.equal(token, env.PASEO_ORCH_TOKEN);
      assert.equal(id, "parent");
      assert.equal(cwd, "/workspace");
      bound = true;
      return true;
    },
  };
  const pending = requireNativeBinding(env, bridge, "parent", "/workspace").then(() => {
    prepared = true;
  });
  await Promise.resolve();
  assert.equal(bound, false);
  assert.equal(prepared, false);
  resolve(env.PASEO_ORCH_URL);
  await pending;
  assert.equal(bound, true);
  assert.equal(prepared, true);
});
test("missing, stale and rejected leases prevent native preparation", async () => {
  for (const [input, url, accepts] of [
    [{}, env.PASEO_ORCH_URL, true],
    [env, "http://127.0.0.1:4321/mcp", true],
    [env, env.PASEO_ORCH_URL, false],
  ] as const) {
    let prepared = false;
    await assert.rejects(async () => {
      await requireNativeBinding(
        input,
        { ready: Promise.resolve(url), bind: () => accepts },
        "parent",
        "/workspace",
      );
      prepared = true;
    }, /bridge binding/);
    assert.equal(prepared, false);
  }
});
test("bridge startup failure hides internal details", async () => {
  await assert.rejects(
    requireNativeBinding(
      env,
      { ready: Promise.reject(new Error("secret-detail")), bind: () => true },
      "parent",
      "/workspace",
    ),
    (error: Error) => {
      assert.match(error.message, /bridge failed to start/);
      assert.doesNotMatch(error.message, /secret-detail/);
      return true;
    },
  );
});
