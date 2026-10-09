import { describe, expect, it } from "vitest";
import { resolveAndValidateCreateAgentMode } from "./create-agent-mode.js";

const CLAUDE_MODES = ["default", "acceptEdits", "plan", "bypassPermissions"];
const OPENCODE_MODES = ["build", "plan"];
const CODEX_MODES = ["auto", "full-access"];

type CreateAgentModeInput = Parameters<typeof resolveAndValidateCreateAgentMode>[0];

// Tests that do not exercise the unattended-mode fallback leave the target without one.
function resolveCreateAgentMode(
  input: Omit<CreateAgentModeInput, "targetUnattendedMode"> &
    Partial<Pick<CreateAgentModeInput, "targetUnattendedMode">>,
) {
  return resolveAndValidateCreateAgentMode({ targetUnattendedMode: undefined, ...input });
}

function agentParent(provider: string, modeId: string | null, isUnattended = false) {
  return { provider, modeId, isUnattended };
}

describe("resolveAndValidateCreateAgentMode", () => {
  it("returns the requested mode when it is valid for the target provider", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: "plan",
      targetProvider: "opencode",
      parent: null,
      unattended: false,
      availableModes: OPENCODE_MODES,
    });
    expect(resolved).toBe("plan");
  });

  it("throws when the requested mode is invalid for the target provider", () => {
    expect(() =>
      resolveCreateAgentMode({
        requestedMode: "bypassPermissions",
        targetProvider: "opencode",
        parent: null,
        unattended: false,
        availableModes: OPENCODE_MODES,
      }),
    ).toThrow(
      "Invalid mode 'bypassPermissions' for provider 'opencode'. Available modes: build, plan",
    );
  });

  it("returns undefined (provider default) when no mode and no caller", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: undefined,
      targetProvider: "claude",
      parent: null,
      unattended: false,
      availableModes: CLAUDE_MODES,
    });
    expect(resolved).toBeUndefined();
  });

  it("inherits the caller mode when caller and target share a provider", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: undefined,
      targetProvider: "claude",
      parent: agentParent("claude", "bypassPermissions"),
      unattended: false,
      availableModes: CLAUDE_MODES,
    });
    expect(resolved).toBe("bypassPermissions");
  });

  it("returns undefined when same-provider caller has no mode", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: undefined,
      targetProvider: "claude",
      parent: agentParent("claude", null),
      unattended: false,
      availableModes: CLAUDE_MODES,
    });
    expect(resolved).toBeUndefined();
  });

  it("refuses cross-provider inheritance with the target provider's modes in the message", () => {
    expect(() =>
      resolveCreateAgentMode({
        requestedMode: undefined,
        targetProvider: "opencode",
        parent: agentParent("claude", "bypassPermissions"),
        unattended: false,
        availableModes: OPENCODE_MODES,
      }),
    ).toThrow(
      "cannot inherit mode 'bypassPermissions' from caller (provider 'claude') for new agent (provider 'opencode'). Pass an explicit mode. Available modes for 'opencode': build, plan",
    );
  });

  it("refuses cross-provider inheritance even when the caller mode is null", () => {
    expect(() =>
      resolveCreateAgentMode({
        requestedMode: undefined,
        targetProvider: "codex",
        parent: agentParent("opencode", null),
        unattended: false,
        availableModes: CODEX_MODES,
      }),
    ).toThrow(
      "cannot inherit mode '<none>' from caller (provider 'opencode') for new agent (provider 'codex'). Pass an explicit mode. Available modes for 'codex': auto, full-access",
    );
  });

  it("uses the provider default when the cross-provider target has no modes", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: undefined,
      targetProvider: "pi",
      parent: agentParent("codex", "auto"),
      unattended: false,
      availableModes: [],
      targetUnattendedMode: undefined,
    });

    expect(resolved).toBeUndefined();
  });

  it("uses the provider default when an unattended parent targets a provider with no modes", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: undefined,
      targetProvider: "pi",
      parent: agentParent("claude", "bypassPermissions", true),
      unattended: false,
      availableModes: [],
      targetUnattendedMode: undefined,
    });

    expect(resolved).toBeUndefined();
  });

  it("passes through an explicit mode when the target provider's modes are unknown", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: "default",
      targetProvider: "zai-custom",
      parent: null,
      unattended: false,
      availableModes: undefined,
    });
    expect(resolved).toBe("default");
  });

  it("renders 'unknown' in cross-provider error when target modes are unknown", () => {
    expect(() =>
      resolveCreateAgentMode({
        requestedMode: undefined,
        targetProvider: "zai-custom",
        parent: agentParent("claude", "default"),
        unattended: false,
        availableModes: undefined,
      }),
    ).toThrow("Available modes for 'zai-custom': unknown");
  });

  it("inherits target's unattended mode when caller is unattended cross-provider", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: undefined,
      targetProvider: "codex",
      parent: agentParent("claude", "bypassPermissions", true),
      unattended: false,
      availableModes: CODEX_MODES,
      targetUnattendedMode: "full-access",
    });
    expect(resolved).toBe("full-access");
  });

  it("inherits target's unattended mode for unattended creation without a parent", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: undefined,
      targetProvider: "codex",
      parent: null,
      unattended: true,
      availableModes: CODEX_MODES,
      targetUnattendedMode: "full-access",
    });
    expect(resolved).toBe("full-access");
  });

  it("still refuses cross-provider inheritance when caller is not unattended", () => {
    expect(() =>
      resolveCreateAgentMode({
        requestedMode: undefined,
        targetProvider: "codex",
        parent: agentParent("claude", "default"),
        unattended: false,
        availableModes: CODEX_MODES,
        targetUnattendedMode: "full-access",
      }),
    ).toThrow(
      "cannot inherit mode 'default' from caller (provider 'claude') for new agent (provider 'codex'). Pass an explicit mode. Available modes for 'codex': auto, full-access",
    );
  });

  it("still refuses cross-provider inheritance when target has no unattended mode", () => {
    expect(() =>
      resolveCreateAgentMode({
        requestedMode: undefined,
        targetProvider: "zai-custom",
        parent: agentParent("claude", "bypassPermissions", true),
        unattended: false,
        availableModes: undefined,
        targetUnattendedMode: undefined,
      }),
    ).toThrow(
      "cannot inherit mode 'bypassPermissions' from caller (provider 'claude') for new agent (provider 'zai-custom'). Pass an explicit mode. Available modes for 'zai-custom': unknown",
    );
  });

  it("explicit mode wins over unattended inheritance", () => {
    const resolved = resolveCreateAgentMode({
      requestedMode: "auto",
      targetProvider: "codex",
      parent: agentParent("claude", "bypassPermissions", true),
      unattended: false,
      availableModes: CODEX_MODES,
      targetUnattendedMode: "full-access",
    });
    expect(resolved).toBe("auto");
  });
});
