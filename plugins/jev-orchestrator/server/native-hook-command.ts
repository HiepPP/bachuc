import { createHash } from "node:crypto";
import { readFile, realpath, open } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { z } from "zod";
import { createJudge } from "./jev";
import { nativePolicySchema, routeNativeHook } from "./native-hook";
import type { Judge } from "./types";

const manifestSchema = z.strictObject({
  cwd: z.string().refine(path.isAbsolute),
  policy: nativePolicySchema,
  definitions: z
    .array(
      z.strictObject({
        agentType: z.string().min(1),
        path: z.string().refine(path.isAbsolute),
        sha256: z.string().regex(/^[a-f0-9]{64}$/),
      }),
    )
    .min(1)
    .max(128),
});

const deny = (
  reason = "Jev native routing unavailable: check the reviewed policy and agent definitions.",
) => ({
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: reason,
  },
});

const FRESH_AGENT = "Create a fresh Paseo agent.";

// Hashes bind routing to reviewed role definitions, including permissions and instructions.
// They do not prove that the provider loaded these files; activation requires a runtime check.
export async function runNativeHookCommand(input: unknown, manifestPath: string, judge: Judge) {
  let reason = `Jev native routing unavailable: policy manifest is unreadable or invalid. ${FRESH_AGENT}`;
  try {
    const manifestText = await readFile(manifestPath, "utf8");
    const manifest = manifestSchema.parse(JSON.parse(manifestText));
    reason = "Native subagent routing denied: hook event has no readable cwd.";
    const event = z.object({ cwd: z.string() }).passthrough().parse(input);
    const workspace = await realpath(manifest.cwd);
    const current = await realpath(event.cwd);
    // Subdirectories stay in scope; only paths outside the agent workspace are denied.
    const relative = path.relative(workspace, current);
    if (relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative))
      return deny(
        `Native subagent routing denied: ${current} is outside the agent workspace ${workspace}.`,
      );
    const candidates = manifest.policy.routes.flatMap((route) => route.candidates);
    if (
      manifest.definitions.length !== candidates.length ||
      new Set(manifest.definitions.map((item) => item.agentType)).size !== candidates.length ||
      candidates.some(
        (candidate) => !manifest.definitions.some((item) => item.agentType === candidate.agentType),
      )
    )
      return deny(
        `Jev native routing unavailable: policy routes do not match their agent definitions. ${FRESH_AGENT}`,
      );
    const verify = async () => {
      for (const definition of manifest.definitions) {
        const contents = await readFile(definition.path);
        if (createHash("sha256").update(contents).digest("hex") !== definition.sha256)
          throw new Error("Definition changed.");
      }
    };
    reason = `Jev native routing unavailable: a generated agent definition changed or is missing. ${FRESH_AGENT}`;
    await verify();
    const result = await routeNativeHook(input, manifest.policy, judge);
    await verify();
    if ((await readFile(manifestPath, "utf8")) !== manifestText)
      return deny(
        `Jev native routing unavailable: policy manifest changed during evaluation. ${FRESH_AGENT}`,
      );
    return result;
  } catch {
    return deny(reason);
  }
}

async function main() {
  let failureReason: string | undefined;
  const manifestPath = process.env.PASEO_JEV_NATIVE_POLICY;
  const runtime = process.env.PASEO_JEV_NATIVE_RUNTIME;
  if (!manifestPath && !runtime) {
    process.stdout.write("{}\n");
    return;
  }
  // Emit a deny before the host's configured 30-second timeout can fail open.
  const timer = setTimeout(() => {
    process.stdout.write(JSON.stringify(deny()) + "\n", () => process.exit(0));
  }, 25000);
  try {
    if (
      !manifestPath ||
      !path.isAbsolute(manifestPath) ||
      !["codex", "claude"].includes(runtime ?? "")
    )
      throw new Error("Paseo native routing scope required.");
    const manifest = manifestSchema.parse(JSON.parse(await readFile(manifestPath, "utf8")));
    if (manifest.policy.runtime !== runtime) throw new Error("Runtime scope mismatch.");
    let raw = "";
    for await (const chunk of process.stdin) {
      raw += chunk.toString();
      if (Buffer.byteLength(raw) > 100000) throw new Error("Input too large.");
    }
    const event = JSON.parse(raw);
    if (
      runtime === "codex" &&
      (event.tool_name === "collaborationspawn_agent" ||
        event.tool_name === "mcp__jev_orchestrator__prepare_native_delegate")
    ) {
      // Native children inherit environment/MCP entries. Only the root transcript
      // may prepare or consume tickets for this Paseo parent.
      if (typeof event.transcript_path !== "string" || typeof event.session_id !== "string")
        throw new Error("Root transcript required.");
      const transcript = await realpath(event.transcript_path);
      const codexHome = await realpath(process.env.CODEX_HOME || path.join(homedir(), ".codex"));
      const relative = path.relative(codexHome, transcript);
      if (
        !relative.startsWith(`sessions${path.sep}`) &&
        !relative.startsWith(`archived_sessions${path.sep}`)
      )
        throw new Error("Invalid transcript scope.");
      const file = await open(transcript, "r");
      let firstLine;
      try {
        const buffer = Buffer.alloc(262144);
        const { bytesRead } = await file.read(buffer, 0, buffer.length, 0);
        const text = buffer.subarray(0, bytesRead).toString("utf8");
        if (!text.includes("\n")) throw new Error("Missing session metadata.");
        firstLine = JSON.parse(text.slice(0, text.indexOf("\n")));
      } finally {
        await file.close();
      }
      const metadata = firstLine.payload;
      if (
        firstLine.type !== "session_meta" ||
        metadata.id !== event.session_id ||
        metadata.parent_thread_id ||
        metadata.thread_source === "subagent" ||
        typeof metadata.source === "object"
      )
        throw new Error("Only the root parent may use native tickets.");
      const url = process.env.PASEO_ORCH_URL;
      const token = process.env.PASEO_ORCH_TOKEN;
      if (
        event.hook_event_name !== "PreToolUse" ||
        !url ||
        !token ||
        !/^http:\/\/127\.0\.0\.1:\d+\/mcp$/.test(url)
      )
        throw new Error("Missing ticket binding.");
      const intent = event.tool_name === "mcp__jev_orchestrator__prepare_native_delegate";
      failureReason =
        "Jev native bridge unreachable. Check jev-orchestrator plugin status; after a plugin reload, create a fresh Paseo agent. Do not restart the daemon.";
      const response = await fetch(url, {
        method: "POST",
        redirect: "error",

        headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
        body: JSON.stringify({
          action: intent ? "native_intent" : "native_consume",
          input: { sessionId: event.session_id, cwd: event.cwd, input: event.tool_input },
        }),
      });
      if (!response.ok) {
        failureReason =
          response.status === 403
            ? "Jev native session binding expired or invalid. Create a fresh Paseo agent; old tickets cannot be reused."
            : "Jev native ticket rejected. Check native_delegation_status, request contract, workspace, policy, expiry and remaining slots. Do not retry with a new request ID.";
        if (response.status === 503) {
          const error = await response.json().catch(() => null);
          if (error?.code === "daemon_disconnected")
            failureReason =
              "Jev native routing unavailable: Paseo daemon transport disconnected. Run paseo plugin reload jev-orchestrator, then create a fresh Paseo agent. Do not restart the daemon.";
        }
        throw new Error("Ticket rejected.");
      }
      failureReason =
        "Jev native bridge returned an invalid response. Check jev-orchestrator plugin logs.";
      const result = await response.json();
      process.stdout.write(JSON.stringify(intent ? {} : result) + "\n");
      return;
    }
    const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
    const configFile = path.join(
      process.env.PASEO_HOME || path.join(homedir(), ".paseo"),
      "config.json",
    );
    const result = await runNativeHookCommand(event, manifestPath, createJudge(configFile, root));
    process.stdout.write(JSON.stringify(result) + "\n");
  } catch {
    process.stdout.write(JSON.stringify(deny(failureReason)) + "\n");
  } finally {
    clearTimeout(timer);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  void main();
