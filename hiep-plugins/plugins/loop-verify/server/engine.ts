import { randomUUID } from "node:crypto";
import { STATUS_OUTPUT_TAIL, type LoopStatus } from "../shared/status";
import type { Loop, Store } from "./store";

export const VERIFY_LABEL = "loop-verify";
export const MAX_LABEL = "loop-verify-max";
export const ROOT_LABEL = "loop-verify-root";
export const DEFAULT_MAX_ROUNDS = 5;
const OUTPUT_TAIL = 6000;

export interface LoopAgent {
  id: string;
  cwd: string;
  provider: string;
  model: string | null;
  modeId: string | null;
  thinkingOptionId: string | null;
  labels: Record<string, string>;
}

export interface CreateRequest {
  agentId: string;
  parentId: string;
  cwd: string;
  config: { provider: string; modeId?: string; thinkingOptionId?: string };
  title: string;
  prompt: string;
  labels: Record<string, string>;
}

export interface Driver {
  inspect(agentId: string): Promise<LoopAgent | null>;
  create(request: CreateRequest): Promise<void>;
  publish(agentId: string, rowId: string, status: LoopStatus): Promise<void>;
}

export interface VerifyResult {
  exitCode: number | null;
  output: string;
}

export type Verify = (command: string, cwd: string) => Promise<VerifyResult>;

export interface TurnEnded {
  agentId: string;
  outcome: "completed" | "failed" | "canceled";
  firstUserMessage: string | null;
  lastAssistantMessage: string | null;
}

export class Engine {
  private readonly ignored = new Set<string>();
  private readonly busy = new Set<string>();

  constructor(
    private readonly store: Store,
    private readonly driver: Driver,
    private readonly verify: Verify,
    private readonly log: (line: string) => void,
    private readonly newId: () => string = randomUUID,
  ) {}

  async ended(event: TurnEnded): Promise<void> {
    const { agentId } = event;
    if (this.busy.has(agentId) || this.ignored.has(agentId)) return;
    this.busy.add(agentId);
    try {
      const loop = this.store.active(agentId) ?? (await this.arm(event));
      if (!loop) return;
      await this.step(loop, event);
    } catch (error) {
      this.log(`agent ${agentId}: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      this.busy.delete(agentId);
    }
  }

  stop(agentId: string): void {
    const loop = this.store.active(agentId) ?? this.store.byRoot(agentId);
    if (!loop || (loop.status !== "running" && loop.status !== "paused")) return;
    loop.status = "stopped";
    this.store.save();
    this.log(`loop ${loop.rootId}: stopped at round ${loop.round}`);
  }

  private async arm(event: TurnEnded): Promise<Loop | null> {
    // A finished loop is not re-armed by follow-up turns on its root agent.
    if (this.store.byRoot(event.agentId)) return null;
    const agent = await this.driver.inspect(event.agentId);
    const command = agent?.labels[VERIFY_LABEL]?.trim();
    if (!agent || !command) {
      this.ignored.add(event.agentId);
      return null;
    }
    const loop: Loop = {
      rootId: agent.id,
      current: agent.id,
      round: 1,
      maxRounds: parseMax(agent.labels[MAX_LABEL]),
      verify: command,
      goal: event.firstUserMessage,
      cwd: agent.cwd,
      config: {
        provider: agent.model ? `${agent.provider}/${agent.model}` : agent.provider,
        ...(agent.modeId ? { modeId: agent.modeId } : {}),
        ...(agent.thinkingOptionId ? { thinkingOptionId: agent.thinkingOptionId } : {}),
      },
      status: "running",
      history: [],
    };
    this.store.put(loop);
    this.log(
      `loop ${loop.rootId}: armed, verify=${JSON.stringify(command)}, max=${loop.maxRounds}`,
    );
    return loop;
  }

  private async step(loop: Loop, event: TurnEnded): Promise<void> {
    if (event.outcome === "canceled") {
      this.stop(event.agentId);
      await this.report(loop, event.agentId, loop.round, null, null);
      return;
    }
    if (loop.status === "paused") {
      // The user answered the paused agent; that turn is this round's new attempt.
      loop.status = "running";
      this.log(`loop ${loop.rootId}: resumed at round ${loop.round}`);
    }
    const result = await this.verify(loop.verify, loop.cwd);
    const round = loop.round;
    loop.history.push({ agentId: event.agentId, round, exitCode: result.exitCode });
    let nextAgentId: string | null = null;
    if (result.exitCode === 0) {
      loop.status = "passed";
      this.log(`loop ${loop.rootId}: GOAL MET at round ${round}`);
    } else if (endsWithQuestion(event.lastAssistantMessage)) {
      loop.status = "paused";
      this.log(
        `loop ${loop.rootId}: paused at round ${round}, agent ${event.agentId} asked a question. Reply to it to resume.`,
      );
    } else if (round >= loop.maxRounds) {
      loop.status = "exhausted";
      this.log(`loop ${loop.rootId}: verify still failing after ${round} rounds`);
    } else {
      loop.round += 1;
      loop.current = nextAgentId = this.newId();
      this.log(`loop ${loop.rootId}: verify exit ${result.exitCode}, starting round ${loop.round}`);
    }
    // Persist the next agent ID before creating it so its turn_ended always resolves to this loop.
    this.store.save();
    await this.report(loop, event.agentId, round, result, nextAgentId);
    if (!nextAgentId) return;
    await this.driver.create({
      agentId: nextAgentId,
      parentId: loop.rootId,
      cwd: loop.cwd,
      config: loop.config,
      title: `loop-verify round ${loop.round}/${loop.maxRounds}`,
      prompt: retryPrompt(loop, result),
      labels: { [ROOT_LABEL]: loop.rootId },
    });
  }

  // Each verify result gets a new row ID so it lands at the end of the timeline; reusing an ID
  // replaces the old row in place, which leaves the latest state above newer messages.
  private async report(
    loop: Loop,
    agentId: string,
    round: number,
    result: VerifyResult | null,
    nextAgentId: string | null,
  ): Promise<void> {
    const status: LoopStatus = {
      rootId: loop.rootId,
      agentId,
      round,
      maxRounds: loop.maxRounds,
      verify: loop.verify,
      exitCode: result?.exitCode ?? null,
      status: loop.status,
      output: result?.output.slice(-STATUS_OUTPUT_TAIL) ?? "",
      nextAgentId,
    };
    const rowId =
      result === null
        ? `loop-verify-${loop.history.length}-stopped`
        : `loop-verify-${loop.history.length}`;
    const targets = agentId === loop.rootId ? [agentId] : [agentId, loop.rootId];
    for (const target of targets) {
      try {
        await this.driver.publish(target, rowId, status);
      } catch (error) {
        this.log(
          `agent ${target}: status row failed: ${error instanceof Error ? error.message : String(error)}`,
        );
      }
    }
  }
}

export function retryPrompt(loop: Loop, result: VerifyResult): string {
  const goal = loop.goal?.trim() || `Make \`${loop.verify}\` pass.`;
  const output = result.output.slice(-OUTPUT_TAIL);
  return [
    goal,
    "",
    `A previous attempt is done but the verify command still fails (round ${loop.round} of ${loop.maxRounds}).`,
    `Command: \`${loop.verify}\` (exit ${result.exitCode ?? "timeout"})`,
    "Last output:",
    "```",
    output,
    "```",
    "Inspect the current working tree, fix the cause, and run the command yourself before finishing.",
  ].join("\n");
}

export function endsWithQuestion(text: string | null): boolean {
  if (!text) return false;
  // Ignore trailing markdown emphasis, code, quotes, and brackets around the final "?".
  return /[?？][\s*_`"')\]]*$/.test(text);
}

function parseMax(value: string | undefined): number {
  const parsed = Number.parseInt(value ?? "", 10);
  if (!Number.isFinite(parsed)) return DEFAULT_MAX_ROUNDS;
  return Math.min(Math.max(parsed, 1), 10);
}
