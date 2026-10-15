import { describe, expect, it } from "vitest";
import { ServerInfoStatusPayloadSchema, SessionInboundMessageSchema } from "./messages";

const baseRequest = {
  type: "fetch_agent_timeline_request",
  agentId: "agent-1",
  requestId: "request-1",
  direction: "tail",
} as const;

const baseServerInfo = {
  status: "server_info",
  serverId: "server-1",
};

describe("timeline latest prompt tail protocol", () => {
  it("parses a fetch request with and without includeLatestPrompt", () => {
    const without = SessionInboundMessageSchema.parse(baseRequest);
    const withFlag = SessionInboundMessageSchema.parse({
      ...baseRequest,
      includeLatestPrompt: true,
    });
    expect(without).not.toHaveProperty("includeLatestPrompt");
    expect(withFlag).toMatchObject({ includeLatestPrompt: true });
  });

  it("parses server_info with and without timelineLatestPromptTail", () => {
    const without = ServerInfoStatusPayloadSchema.parse(baseServerInfo);
    const withFlag = ServerInfoStatusPayloadSchema.parse({
      ...baseServerInfo,
      features: { timelineLatestPromptTail: true },
    });
    expect(without.features?.timelineLatestPromptTail).toBeUndefined();
    expect(withFlag.features?.timelineLatestPromptTail).toBe(true);
  });
});
