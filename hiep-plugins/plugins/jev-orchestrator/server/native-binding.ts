type BindingEnv = { PASEO_ORCH_URL?: string; PASEO_ORCH_TOKEN?: string };
const fresh = "Create a fresh Paseo agent.";

export function nativeBindingError(env: BindingEnv): string | undefined {
  const missing = ["PASEO_ORCH_URL", "PASEO_ORCH_TOKEN"].filter(
    (key) => !env[key as keyof BindingEnv],
  );
  if (missing.length)
    return `Jev native routing unavailable: missing bridge binding (${missing.join(", ")}). ${fresh}`;
  if (!/^http:\/\/127\.0\.0\.1:\d+\/mcp$/.test(env.PASEO_ORCH_URL!))
    return `Jev native routing unavailable: invalid bridge URL. ${fresh}`;
}

export async function requireNativeBinding(
  env: BindingEnv,
  bridge: { ready: Promise<string>; bind(token: string, parentId: string, cwd: string): boolean },
  parentId: string,
  cwd: string,
): Promise<void> {
  const error = nativeBindingError(env);
  if (error) throw new Error(error);
  let url: string;
  try {
    url = await bridge.ready;
  } catch {
    throw new Error(
      `Jev native routing unavailable: bridge failed to start. Reload jev-orchestrator, then create a fresh Paseo agent.`,
    );
  }
  if (env.PASEO_ORCH_URL !== url || !bridge.bind(env.PASEO_ORCH_TOKEN!, parentId, cwd))
    throw new Error(`Jev native routing unavailable: bridge binding expired or invalid. ${fresh}`);
}
