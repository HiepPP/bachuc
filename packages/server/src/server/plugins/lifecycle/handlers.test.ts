import { expect, test } from "vitest";
import { createPaseoApi } from "@getpaseo/client";
import { DaemonClient } from "@getpaseo/client/internal/daemon-client";
import { PluginHookHandlers } from "./index.js";

const paseo = createPaseoApi(
  new DaemonClient({ url: "ws://127.0.0.1:1/ws", clientId: "lifecycle-unit" }),
);

test("removing an old registration twice preserves a newer registration for the same hook", async () => {
  const hooks = new PluginHookHandlers(() => {});
  const remove = hooks.before("workspace.create", ({ request }) => {
    return { ...request, title: "old" };
  });
  remove();
  hooks.before("workspace.create", ({ request }) => {
    return { ...request, title: "new" };
  });
  remove();
  const output = await hooks.invoke(
    "operation",
    "before",
    "workspace.create",
    {
      source: { kind: "directory", path: "/project" },
    },
    paseo,
  );
  expect(output).toEqual({ source: { kind: "directory", path: "/project" }, title: "new" });
});

test("before hooks compose returned requests and preserve the original input", async () => {
  const hooks = new PluginHookHandlers(() => {});
  hooks.before("workspace.create", ({ request }) => {
    return { ...request, title: "first" };
  });
  hooks.before("workspace.create", () => {
    return;
  });
  hooks.before("workspace.create", ({ request }) => {
    return { ...request, title: request.title + ":second" };
  });
  const input = { source: { kind: "directory", path: "/project" } };
  expect(await hooks.invoke("operation", "before", "workspace.create", input, paseo)).toEqual({
    source: { kind: "directory", path: "/project" },
    title: "first:second",
  });
  expect(input).toEqual({ source: { kind: "directory", path: "/project" } });
});

test("teardown aborts an active callback and removes its registrations", async () => {
  const hooks = new PluginHookHandlers(() => {});
  hooks.before("workspace.create", async (_input, context) => {
    await new Promise<void>((_resolve, reject) => {
      context.signal.addEventListener(
        "abort",
        () => {
          reject(new Error("Hook aborted"));
        },
        { once: true },
      );
    });
  });
  const invocation = hooks.invoke(
    "operation",
    "before",
    "workspace.create",
    {
      source: { kind: "directory", path: "/project" },
    },
    paseo,
  );
  hooks.close();
  await expect(invocation).rejects.toThrow("Hook aborted");
  expect(hooks.catalog()).toEqual({ events: [], before: [] });
});

test("session-open hooks reject changes to session identity instead of silently ignoring them", async () => {
  const hooks = new PluginHookHandlers(() => {});
  hooks.before("agent.session_open", ({ request }) => {
    return { ...request, provider: "another-provider" };
  });
  await expect(
    hooks.invoke(
      "operation",
      "before",
      "agent.session_open",
      {
        agentId: "agent",
        workspaceId: "workspace",
        provider: "claude",
        cwd: "/project",
        reason: "resume",
        purpose: "interactive",
        env: {},
      },
      paseo,
    ),
  ).rejects.toThrow("agent.session_open hooks can only change env");
});

const promptRequest = {
  agentId: "agent-1",
  workspaceId: null,
  provider: "claude",
  cwd: "/project",
  kind: "turn" as const,
  prompt: "Hello",
};

test("prompt hooks rewrite only the provider prompt", async () => {
  const hooks = new PluginHookHandlers(() => {});
  hooks.before("agent.prompt", ({ request }) => {
    return { ...request, prompt: `${request.prompt as string}\n\nContext` };
  });
  expect(await hooks.invoke("operation", "before", "agent.prompt", promptRequest, paseo)).toEqual({
    ...promptRequest,
    prompt: "Hello\n\nContext",
  });
});

test("prompt hooks cannot move the prompt to another agent", async () => {
  const hooks = new PluginHookHandlers(() => {});
  hooks.before("agent.prompt", ({ request }) => {
    return { ...request, agentId: "agent-2" };
  });
  await expect(
    hooks.invoke("operation", "before", "agent.prompt", promptRequest, paseo),
  ).rejects.toThrow("agent.prompt hooks can only change prompt");
});

const permissionRequest = {
  agentId: "agent-1",
  workspaceId: "workspace-1",
  provider: "claude",
  cwd: "/project",
  request: { id: "permission-1", provider: "claude", name: "Bash", kind: "tool" as const },
  decision: null,
};

test("permission hooks set a decision", async () => {
  const hooks = new PluginHookHandlers(() => {});
  hooks.before("agent.permission", ({ request }) => {
    return { ...request, decision: { behavior: "allow" as const } };
  });
  expect(
    await hooks.invoke("operation", "before", "agent.permission", permissionRequest, paseo),
  ).toEqual({ ...permissionRequest, decision: { behavior: "allow" } });
});

test("permission hooks cannot change the request", async () => {
  const hooks = new PluginHookHandlers(() => {});
  hooks.before("agent.permission", ({ request }) => {
    return { ...request, request: { ...request.request, name: "Edit" } };
  });
  await expect(
    hooks.invoke("operation", "before", "agent.permission", permissionRequest, paseo),
  ).rejects.toThrow("agent.permission hooks can only change decision");
});
