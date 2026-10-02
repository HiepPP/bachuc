// Copied from @getpaseo/protocol/agent-labels: a value import of that package would have to be
// installed next to a bundled plugin, where only production dependencies exist.
const PARENT_AGENT_ID_LABEL = "paseo.parent-agent-id";

export function getParentAgentIdFromLabels(labels: Record<string, unknown> | null | undefined) {
  const parentAgentId = labels?.[PARENT_AGENT_ID_LABEL];
  return typeof parentAgentId === "string" && parentAgentId.trim().length > 0
    ? parentAgentId.trim()
    : null;
}
