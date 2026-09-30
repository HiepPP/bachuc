import type { Task } from "../shared/contracts";
import type { TokenUsage } from "./usage";

export type Profile = {
  id: string;
  name: string;
  provider: string;
  model: string;
  modeId?: string;
  thinkingOptionId?: string;
  featureValues?: Record<string, unknown>;
  notes?: string;
};
export type Phase = "discovery" | "implementation" | "review";
export type Check = {
  argv: string[];
  exitCode: number | null;
  output: string;
  durationMs: number;
  timedOut: boolean;
};
export type Attempt = {
  childId: string;
  phase: Phase;
  profile: Profile;
  startedAt: number;
  endedAt?: number;
  status?: string;
  output?: string;
  checks?: Check[];
  costUsd?: number;
  tokens?: number;
  usage?: TokenUsage;
  observed?: {
    provider: string;
    model?: string | null;
    thinkingOptionId?: string | null;
    modeId?: string | null;
  };
};
export type Decision = {
  profileId: string;
  discovery: boolean;
  risk: "low" | "high";
  category: string;
  recovery?: "retry" | "escalate" | "environment" | "needs_input";
  reviewPassed?: boolean;
  probabilities?: Record<string, number>;
  usage?: TokenUsage;
};
export type Job = {
  key: string;
  parentId: string;
  cwd: string;
  task: Task;
  status:
    | "queued"
    | "running"
    | "passed"
    | "unverified"
    | "failed"
    | "needs_input"
    | "cancelled"
    | "interrupted";
  phase?: Phase;
  createdAt: number;
  startedAt?: number;
  endedAt?: number;
  decision?: Decision;
  attempts: Attempt[];
  evaluations?: {
    phase: "route" | "recovery" | "review";
    startedAt: number;
    endedAt?: number;
    usage: TokenUsage;
    error?: string;
  }[];
  message: string;
  notifyError?: string;
  notificationCompleteAt?: number;
};
export type Metric = {
  profileId: string;
  profileVersion: string;
  category: string;
  phase: Phase;
  verified: boolean;
  durationMs: number;
  costUsd?: number;
  tokens?: number;
  at: number;
};
export type Driver = {
  profiles(cwd: string): Promise<Profile[]>;
  launch(job: Job, profile: Profile, phase: Phase, prompt: string): Promise<string>;
  wait(
    childId: string,
    timeoutMs: number,
  ): Promise<{
    status: string;
    output: string;
    costUsd?: number;
    tokens?: number;
    usage?: TokenUsage;
    observed?: Attempt["observed"];
  }>;
  archive(childId: string): Promise<void>;
  notify(job: Job): Promise<void>;
};
export type Judge = (
  phase: "route" | "recovery" | "review" | "direct",
  state: Record<string, unknown>,
  profiles: Profile[],
  signal: AbortSignal,
) => Promise<Decision>;
