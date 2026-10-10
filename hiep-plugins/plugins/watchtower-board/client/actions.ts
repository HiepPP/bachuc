import type { PaseoAgent } from "@getpaseo/client";

// The board edits no file. Each action sends one of these prompts to an agent of the workspace,
// which changes the files through the watchtower skill.
// Answers, defaults, and check texts are sentences. Without their final period, the prompt does not
// end a clause with two.
function bare(text: string): string {
  return text.trim().replace(/\.+$/, "");
}

export function answerPrompt(id: string, question: string, answer: string): string {
  return `Watchtower: the owner answers ${id} ("${question}"): ${bare(answer)}. Use the watchtower skill: write the answer into its watchtower/QUESTIONS.md row, set ANSWERED, add it to the Boundaries of the specs it changes, and unblock the tasks it blocked.`;
}

export function defaultAnswerPrompt(id: string, question: string, defaultAnswer: string): string {
  return answerPrompt(id, question, `use the default: ${defaultAnswer}`);
}

export function acceptDecisionPrompt(id: string, title: string): string {
  return `Watchtower: the owner accepts the proposed ${id} (${title}). Use the watchtower skill decision rules: set Status: accepted in the ADR file and its watchtower/DECISIONS.md row, and add its ## Avoid item to watchtower/MEMORY.md (R4).`;
}

export function rejectDecisionPrompt(id: string, title: string): string {
  return `Watchtower: the owner rejects the proposed ${id} (${title}). Use the watchtower skill decision rules: delete the draft ADR file and its watchtower/DECISIONS.md index row.`;
}

export function manualChecksPrompt(checks: readonly string[]): string {
  return `Watchtower: the owner ran the pending manual checks: ${checks.map(bare).join("; ")}. Use the watchtower skill: record them as done in the Handoff of watchtower/NEXT.md.`;
}

export type AgentCandidate = Pick<
  PaseoAgent,
  "id" | "title" | "status" | "workspaceId" | "archivedAt" | "updatedAt" | "labels"
>;

export type AgentTarget =
  | { state: "ready"; id: string; name: string }
  // Every agent of the workspace has a turn in flight.
  | { state: "busy" }
  | { state: "none" };

// Sending to an agent with a turn in flight interrupts that turn (the daemon default), and the
// client cannot ask to steer instead. So a prompt goes to the newest agent that is not working.
const WORKING = new Set<AgentCandidate["status"]>(["running", "initializing"]);
const PARENT_LABEL = "paseo.parent-agent-id";

export function chooseAgent(agents: readonly AgentCandidate[], workspaceId: string): AgentTarget {
  // A subagent belongs to its parent's turn, so it is not a target.
  const own = agents
    .filter(
      (agent) =>
        agent.workspaceId === workspaceId && !agent.archivedAt && !agent.labels?.[PARENT_LABEL],
    )
    .sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt));
  if (own.length === 0) return { state: "none" };
  const idle = own.find((agent) => !WORKING.has(agent.status));
  return idle
    ? { state: "ready", id: idle.id, name: idle.title?.trim() || idle.id }
    : { state: "busy" };
}
