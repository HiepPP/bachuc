import type { Logger } from "pino";

import type {
  AgentPermissionRequest,
  AgentPromptInput,
  AgentRunOptions,
} from "./agent-sdk-types.js";
import type { AgentManager, ManagedAgent } from "./agent-manager.js";
import type { AgentStorage } from "./agent-storage.js";
import { ensureAgentLoaded } from "./agent-loading.js";
import { isStaleProviderSessionError } from "./stale-provider-session-error.js";
import { getParentAgentIdFromLabels } from "@getpaseo/protocol/agent-labels";
import type { ActiveTurnBehavior } from "@getpaseo/protocol/messages";

export type AgentUnarchiveController = Pick<AgentManager, "notifyAgentState" | "unarchiveSnapshot">;

export type AgentRunController = Pick<
  AgentManager,
  | "getAgent"
  | "tryRunOutOfBand"
  | "hasInFlightRun"
  | "replaceAgentRun"
  | "steerOrReplaceActiveTurn"
  | "streamAgent"
> & {
  reloadAgentSession(agentId: string): Promise<unknown>;
};

export interface StartAgentRunOptions {
  replaceRunning?: boolean;
  activeTurnBehavior?: ActiveTurnBehavior;
  runOptions?: AgentRunOptions;
  /** Ask the provider to deny permissions blocking this steer. */
  clearPendingPermissions?: boolean;
}

export type PromptDispatchDisposition = "out_of_band" | "steered" | "turn_started";

async function steerOrReplaceActiveRun(
  agentManager: AgentRunController,
  agentId: string,
  prompt: AgentPromptInput,
  options: StartAgentRunOptions | undefined,
): Promise<
  | { disposition: "steered" }
  | {
      disposition: "turn_started";
      iterator: AsyncGenerator<import("./agent-sdk-types.js").AgentStreamEvent>;
    }
  | null
> {
  if (options?.activeTurnBehavior !== "steer") {
    return null;
  }
  const steerOptions = options.clearPendingPermissions
    ? { ...options.runOptions, clearPendingPermissions: true }
    : options.runOptions;
  const result = await agentManager.steerOrReplaceActiveTurn(agentId, prompt, steerOptions);
  if (result.status === "steered") {
    return { disposition: "steered" };
  }
  if (result.status === "replaced") {
    return { disposition: "turn_started", iterator: result.iterator };
  }
  return null;
}

async function startOrReplaceRun(
  agentManager: AgentRunController,
  agentId: string,
  prompt: AgentPromptInput,
  options: StartAgentRunOptions | undefined,
): Promise<{
  iterator: AsyncGenerator<import("./agent-sdk-types.js").AgentStreamEvent>;
  replaced: boolean;
}> {
  const replaced = Boolean(options?.replaceRunning && agentManager.hasInFlightRun(agentId));
  const iterator = replaced
    ? await agentManager.replaceAgentRun(agentId, prompt, options?.runOptions)
    : agentManager.streamAgent(agentId, prompt, options?.runOptions);
  return { iterator, replaced };
}

async function drainAgentRunIterator(
  iterator: AsyncGenerator<import("./agent-sdk-types.js").AgentStreamEvent>,
): Promise<void> {
  for await (const _ of iterator) {
    // Events are broadcast via AgentManager subscribers.
  }
}

export async function startAgentRun(
  agentManager: AgentRunController,
  agentId: string,
  prompt: AgentPromptInput,
  logger: Logger,
  options?: StartAgentRunOptions,
): Promise<{ disposition: PromptDispatchDisposition }> {
  const snapshot = agentManager.getAgent(agentId);
  logger.trace(
    {
      agentId,
      provider: snapshot?.provider,
      providerSessionId: snapshot?.persistence?.sessionId ?? undefined,
      turnId: snapshot?.activeForegroundTurnId ?? undefined,
      promptType: typeof prompt === "string" ? "string" : "structured",
      hasRunOptions: Boolean(options?.runOptions),
      replaceRunning: Boolean(options?.replaceRunning),
    },
    "agent.session.start_stream.request",
  );
  // Out-of-band commands (e.g. /goal pause) must run WITHOUT canceling an
  // in-flight turn — replaceAgentRun would interrupt the running turn. The
  // intercept lives at this layer so it covers every prompt entrypoint.
  if (agentManager.tryRunOutOfBand(agentId, prompt, options?.runOptions)) {
    return { disposition: "out_of_band" };
  }
  try {
    return await startAgentRunInner(agentManager, agentId, prompt, logger, options);
  } catch (error) {
    if (!isStaleProviderSessionError(error)) throw error;
    logger.info({ agentId, err: error }, "Provider session went stale; reopening from persistence");
    // The live session belongs to a retired plugin runtime. Reload swaps in a
    // fresh session on the current runtime while preserving history and labels.
    await agentManager.reloadAgentSession(agentId);
    return await startAgentRunInner(agentManager, agentId, prompt, logger, options);
  }
}

async function startAgentRunInner(
  agentManager: AgentRunController,
  agentId: string,
  prompt: AgentPromptInput,
  logger: Logger,
  options?: StartAgentRunOptions,
): Promise<{ disposition: PromptDispatchDisposition }> {
  const snapshot = agentManager.getAgent(agentId);
  const steered = await steerOrReplaceActiveRun(agentManager, agentId, prompt, options);
  if (steered?.disposition === "steered") {
    return steered;
  }
  const { iterator, replaced } = steered
    ? { iterator: steered.iterator, replaced: true }
    : await startOrReplaceRun(agentManager, agentId, prompt, options);
  logger.trace(
    {
      agentId,
      provider: snapshot?.provider,
      providerSessionId: snapshot?.persistence?.sessionId ?? undefined,
      shouldReplace: replaced,
    },
    "agent.session.start_stream.iterator_returned",
  );
  void (async () => {
    try {
      try {
        await drainAgentRunIterator(iterator);
      } catch (error) {
        if (!isStaleProviderSessionError(error)) throw error;
        logger.info(
          { agentId, err: error },
          "Provider session went stale; reopening from persistence",
        );
        await agentManager.reloadAgentSession(agentId);
        const retry = await startOrReplaceRun(agentManager, agentId, prompt, options);
        await drainAgentRunIterator(retry.iterator);
      }
      logger.trace(
        {
          agentId,
          provider: snapshot?.provider,
          providerSessionId: snapshot?.persistence?.sessionId ?? undefined,
        },
        "agent.session.iterator.drained",
      );
    } catch (error) {
      logger.trace(
        {
          agentId,
          provider: snapshot?.provider,
          providerSessionId: snapshot?.persistence?.sessionId ?? undefined,
          err: error,
        },
        "agent.session.iterator.error",
      );
      logger.error({ err: error, agentId }, "Agent stream failed");
    }
  })();
  return { disposition: "turn_started" };
}

/**
 * Clear the archived flag from a stored agent record.
 * Shared across Session (app/WS), MCP, and CLI so every surface that acts on
 * an archived agent unarchives it the same way.
 */
export async function unarchiveAgentState(
  _agentStorage: AgentStorage,
  agentManager: AgentUnarchiveController,
  agentId: string,
  updates?: { workspaceId?: string; labels?: Record<string, string | null> },
): Promise<boolean> {
  const unarchived = await agentManager.unarchiveSnapshot(agentId, updates);
  if (!unarchived) return false;
  agentManager.notifyAgentState(agentId);
  return true;
}

/**
 * Wrap a body in <paseo-system>…</paseo-system> so the receiving agent
 * recognizes the prompt as system-injected context — not a user turn.
 * Used by chat mentions, schedule fires, and notify-on-finish.
 */
export function formatSystemNotificationPrompt(reason: string): string {
  return `<paseo-system>\n${reason}\n</paseo-system>`;
}

const SYSTEM_ENVELOPE_PATTERN = /^<paseo-system>\n[\s\S]*\n<\/paseo-system>$/;

export function isSystemInjectedEnvelope(text: string): boolean {
  return SYSTEM_ENVELOPE_PATTERN.test(text);
}

export interface SendPromptToAgentParams {
  agentManager: AgentManager;
  agentStorage: AgentStorage;
  agentId: string;
  /** Prompt to dispatch to the provider (may include image blocks or wrapped text). */
  prompt: AgentPromptInput;
  messageId?: string;
  activeTurnBehavior?: ActiveTurnBehavior;
  runOptions?: AgentRunOptions;
  /** Optional mode to set on the agent before the run starts. */
  sessionMode?: string;
  /**
   * Default true. When false, archived agents are skipped instead of being
   * unarchived. Use false for system-injected prompts (chat mentions,
   * schedule fires, notify-on-finish).
   */
  unarchive?: boolean;
  /** See {@link StartAgentRunOptions.clearPendingPermissions}. */
  clearPendingPermissions?: boolean;
  /** Default true. When false, a run already in flight makes the send fail instead. */
  replaceRunning?: boolean;
  logger: Logger;
}

export interface StartCreatedAgentInitialPromptParams {
  agentManager: AgentManager;
  agentId: string;
  snapshot?: ManagedAgent;
  prompt: AgentPromptInput | null;
  runOptions?: AgentRunOptions;
  logger: Logger;
}

/**
 * Outer bound on a run reaching "started" after dispatch.
 *
 * This wraps provider startup, so it MUST stay larger than the slowest provider's own
 * startup budget — otherwise it aborts a start the provider was still allowed to be
 * working on, and the provider's budget can never apply. OpenCode is the slowest today:
 * up to 30s for the server to boot (OPENCODE_SERVER_STARTUP_TIMEOUT_MS) and then a
 * session.create on the same budget, so this is deliberately set well above 30s.
 *
 * Not derived from the provider constant on purpose: this module is provider-agnostic
 * and must not depend on a specific provider's internals.
 */
const AGENT_RUN_START_TIMEOUT_MS = 60_000;

export async function waitForAgentRunStartWithTimeout(
  agentManager: AgentManager,
  agentId: string,
  signal?: AbortSignal,
): Promise<void> {
  const provider = agentManager.getAgent(agentId)?.provider ?? "provider";
  const startAbort = new AbortController();
  const startTimeout = setTimeout(
    () =>
      startAbort.abort(
        new Error(
          `${provider} run did not start within ${AGENT_RUN_START_TIMEOUT_MS / 1000} seconds (phase: run start)`,
        ),
      ),
    AGENT_RUN_START_TIMEOUT_MS,
  );

  try {
    await agentManager.waitForAgentRunStart(agentId, {
      signal: signal ? AbortSignal.any([startAbort.signal, signal]) : startAbort.signal,
    });
  } finally {
    clearTimeout(startTimeout);
  }
}

/**
 * Full send-prompt orchestration: (optional unarchive) → load → (optional
 * mode change) → start run.
 *
 * Every surface that sends a prompt to an agent (Session/WS, MCP, CLI-through-MCP,
 * chat mentions, notify-on-finish) MUST go through this so behavior can never
 * drift between them.
 *
 * When `unarchive` is false and the agent is archived, the call is a silent
 * no-op (returns the normal turn-start disposition) — the agent is not run.
 */
export async function sendPromptToAgent(
  params: SendPromptToAgentParams,
): Promise<{ disposition: PromptDispatchDisposition }> {
  const unarchive = params.unarchive ?? true;

  const record = await params.agentStorage.get(params.agentId);
  if (record?.archivedAt) {
    if (!unarchive) {
      return { disposition: "turn_started" };
    }
    await unarchiveAgentState(params.agentStorage, params.agentManager, params.agentId);
  }

  await ensureAgentLoaded(params.agentId, {
    agentManager: params.agentManager,
    agentStorage: params.agentStorage,
    logger: params.logger,
  });

  if (params.sessionMode) {
    await params.agentManager.setAgentMode(params.agentId, params.sessionMode);
  }

  const runOptions = params.messageId
    ? { ...params.runOptions, clientMessageId: params.messageId }
    : params.runOptions;

  return await startAgentRun(params.agentManager, params.agentId, params.prompt, params.logger, {
    replaceRunning: params.replaceRunning ?? true,
    activeTurnBehavior: params.activeTurnBehavior,
    clearPendingPermissions: params.clearPendingPermissions,
    runOptions,
  });
}

export async function startCreatedAgentInitialPrompt(
  params: StartCreatedAgentInitialPromptParams,
): Promise<ManagedAgent> {
  const currentSnapshot = params.agentManager.getAgent(params.agentId) ?? params.snapshot ?? null;
  if (!currentSnapshot) {
    throw new Error(`Agent ${params.agentId} not found`);
  }

  if (params.prompt === null) {
    return currentSnapshot;
  }

  const dispatchResult = await startAgentRun(
    params.agentManager,
    params.agentId,
    params.prompt,
    params.logger,
    {
      runOptions: params.runOptions,
    },
  );

  if (dispatchResult.disposition === "turn_started") {
    await waitForAgentRunStartWithTimeout(params.agentManager, params.agentId);
  }

  const refreshedSnapshot = params.agentManager.getAgent(params.agentId) ?? params.snapshot ?? null;
  if (!refreshedSnapshot) {
    throw new Error(`Agent ${params.agentId} not found`);
  }
  return refreshedSnapshot;
}

/**
 * When a caller hears about its children. `always` batches notices and steers them into a
 * running turn. `settled_only` holds them until the caller's own run ends.
 */
export type FinishNotificationWake = "always" | "settled_only";

export const FINISH_NOTICE_WINDOW_MS = 1500;

export interface SetupFinishNotificationParams {
  agentManager: AgentManager;
  agentStorage: AgentStorage;
  childAgentId: string;
  callerAgentId: string;
  requireParentOwnership?: boolean;
  wake?: FinishNotificationWake;
  /** Defaults to {@link FINISH_NOTICE_WINDOW_MS}. */
  coalesceWindowMs?: number;
  logger: Logger;
}

type FinishNotificationReason = "finished" | "errored" | "needs permission" | "was closed";

const FINISH_NOTIFICATION_MESSAGE_LIMIT = 4000;

interface FinishNotificationBodyInput {
  childAgentId: string;
  title: string;
  reason: FinishNotificationReason;
  lastAssistantMessage: string | null;
  permissionRequest?: AgentPermissionRequest;
}

function formatFinishNotificationBody(params: FinishNotificationBodyInput): string {
  const statusLine = `Agent ${params.childAgentId} (${params.title}) ${params.reason}.`;
  const sections = [statusLine];
  if (params.reason === "needs permission" && params.permissionRequest) {
    sections.push(
      "Respond with `respond_to_permission` using the `agentId` and `requestId` below.",
      `<permission-request>\n${JSON.stringify(
        {
          agentId: params.childAgentId,
          requestId: params.permissionRequest.id,
          request: params.permissionRequest,
        },
        null,
        2,
      )}\n</permission-request>`,
    );
  }
  let lastAssistantMessage = params.lastAssistantMessage?.trim();
  if (lastAssistantMessage) {
    if (lastAssistantMessage.length > FINISH_NOTIFICATION_MESSAGE_LIMIT) {
      const omitted = lastAssistantMessage.length - FINISH_NOTIFICATION_MESSAGE_LIMIT;
      lastAssistantMessage = `${lastAssistantMessage.slice(0, FINISH_NOTIFICATION_MESSAGE_LIMIT)}\n[truncated ${omitted} chars; use get_agent_activity for the full response]`;
    }
    sections.push(`<agent-response>\n${lastAssistantMessage}\n</agent-response>`);
  }
  return sections.join("\n\n");
}

interface NotifySafelyOptions {
  terminal?: boolean;
  permissionRequest?: AgentPermissionRequest;
}

interface ArmedFinishNotification {
  callerAgentId: string;
  stop: () => void;
}

// A caller waits on a child through one armed notification. Arming again, such as a
// follow-up prompt while the child still runs, replaces the earlier one so the child's
// next finish reaches the caller once. Keyed by the manager object so the lifecycle
// commands can disarm through their narrower manager interface.
const armedFinishNotifications = new WeakMap<object, Map<string, ArmedFinishNotification>>();

interface PendingFinishDelivery {
  callerAgentId: string;
  cancelled: boolean;
}

// A terminal notice leaves the armed map before its delivery finishes reading storage.
// Disarm also cancels these in-flight deliveries, or a notice could land after Stop.
const pendingFinishDeliveries = new WeakMap<object, Set<PendingFinishDelivery>>();

interface FinishNoticeEntry {
  key: string;
  childAgentId: string;
  reason: FinishNotificationReason;
  body: string;
}

interface FinishNoticeMailbox {
  callerAgentId: string;
  wake: FinishNotificationWake;
  entries: FinishNoticeEntry[];
  timer: ReturnType<typeof setTimeout> | null;
  unsubscribeSettle: (() => void) | null;
}

// Children that settle close together reach their caller as one system message. One
// mailbox per caller and wake mode, so a settled-only notice never waits behind, or
// rides along with, a steered one.
const finishNoticeMailboxes = new WeakMap<object, Map<string, FinishNoticeMailbox>>();

interface EnqueueFinishNoticeInput {
  agentManager: AgentManager;
  agentStorage: AgentStorage;
  logger: Logger;
  callerAgentId: string;
  wake: FinishNotificationWake;
  windowMs: number;
  entry: FinishNoticeEntry;
  immediate: boolean;
}

function enqueueFinishNotice(input: EnqueueFinishNoticeInput): void {
  const mailboxes =
    finishNoticeMailboxes.get(input.agentManager) ?? new Map<string, FinishNoticeMailbox>();
  finishNoticeMailboxes.set(input.agentManager, mailboxes);
  const mailboxKey = JSON.stringify([input.callerAgentId, input.wake]);
  let mailbox = mailboxes.get(mailboxKey);
  if (!mailbox) {
    mailbox = {
      callerAgentId: input.callerAgentId,
      wake: input.wake,
      entries: [],
      timer: null,
      unsubscribeSettle: null,
    };
    mailboxes.set(mailboxKey, mailbox);
  }
  // The same child and reason again in one window, such as after a quick follow-up,
  // keeps one section with the newest body.
  const duplicateIndex = mailbox.entries.findIndex((existing) => existing.key === input.entry.key);
  if (duplicateIndex === -1) {
    mailbox.entries.push(input.entry);
  } else {
    mailbox.entries[duplicateIndex] = input.entry;
  }

  const flushInput = { ...input, mailbox, mailboxes, mailboxKey };
  if (input.immediate) {
    flushFinishNotices(flushInput);
    return;
  }
  if (!mailbox.timer) {
    mailbox.timer = setTimeout(() => flushFinishNotices(flushInput), input.windowMs);
  }
}

interface FlushFinishNoticesInput extends EnqueueFinishNoticeInput {
  mailbox: FinishNoticeMailbox;
  mailboxes: Map<string, FinishNoticeMailbox>;
  mailboxKey: string;
}

function isCallerRunning(agentManager: AgentManager, callerAgentId: string): boolean {
  return (
    agentManager.hasInFlightRun(callerAgentId) ||
    agentManager.getAgent(callerAgentId)?.lifecycle === "running"
  );
}

function closeFinishNoticeMailbox(input: FlushFinishNoticesInput): void {
  const { mailbox } = input;
  if (mailbox.timer) {
    clearTimeout(mailbox.timer);
    mailbox.timer = null;
  }
  mailbox.unsubscribeSettle?.();
  mailbox.unsubscribeSettle = null;
  if (input.mailboxes.get(input.mailboxKey) === mailbox) {
    input.mailboxes.delete(input.mailboxKey);
  }
}

function flushFinishNotices(input: FlushFinishNoticesInput): void {
  const { agentManager, mailbox } = input;
  if (mailbox.entries.length === 0) {
    closeFinishNoticeMailbox(input);
    return;
  }
  if (mailbox.timer) {
    clearTimeout(mailbox.timer);
    mailbox.timer = null;
  }
  // A permission request blocks the child, so it never waits for the caller to settle.
  const holdsPermission = mailbox.entries.some((entry) => entry.reason === "needs permission");
  if (
    mailbox.wake === "settled_only" &&
    !holdsPermission &&
    isCallerRunning(agentManager, mailbox.callerAgentId)
  ) {
    mailbox.unsubscribeSettle ??= agentManager.subscribe(
      (event) => {
        if (event.type !== "agent_state" || event.agent.id !== mailbox.callerAgentId) {
          return;
        }
        if (!isCallerRunning(agentManager, mailbox.callerAgentId)) {
          flushFinishNotices(input);
        } else if (event.agent.lifecycle !== "running") {
          // A failed turn start emits its error state before it clears the in-flight run,
          // and no later state event follows. Check again once the run has settled.
          setTimeout(() => {
            if (!isCallerRunning(agentManager, mailbox.callerAgentId)) {
              flushFinishNotices(input);
            }
          }, 0);
        }
      },
      { agentId: mailbox.callerAgentId, replayState: false },
    );
    return;
  }

  const entries = mailbox.entries.splice(0);
  closeFinishNoticeMailbox(input);
  const body = entries.map((entry) => entry.body).join("\n\n");
  void sendPromptToAgent({
    agentManager,
    agentStorage: input.agentStorage,
    agentId: mailbox.callerAgentId,
    prompt: formatSystemNotificationPrompt(body),
    activeTurnBehavior: "steer",
    unarchive: false,
    logger: input.logger,
  }).then(
    () => undefined,
    (error: unknown) => {
      const [first] = entries;
      input.logger.error(
        entries.length === 1 && first
          ? {
              err: error,
              childAgentId: first.childAgentId,
              callerAgentId: mailbox.callerAgentId,
              reason: first.reason,
            }
          : {
              err: error,
              childAgentIds: entries.map((entry) => entry.childAgentId),
              callerAgentId: mailbox.callerAgentId,
              reasons: entries.map((entry) => entry.reason),
            },
        "Failed to notify caller agent",
      );
    },
  );
}

export interface DisarmFinishNotificationsInput {
  agentManager: object;
  callerAgentIds: ReadonlySet<string>;
}

// Stop drops every notice still armed or in flight for the stopped agents, so a child
// that settles after the stop cannot start a new turn on a caller the user just stopped.
export function disarmFinishNotifications(input: DisarmFinishNotificationsInput): void {
  const armedByManager = armedFinishNotifications.get(input.agentManager);
  for (const armed of Array.from(armedByManager?.values() ?? [])) {
    if (input.callerAgentIds.has(armed.callerAgentId)) {
      armed.stop();
    }
  }
  for (const delivery of pendingFinishDeliveries.get(input.agentManager) ?? []) {
    if (input.callerAgentIds.has(delivery.callerAgentId)) {
      delivery.cancelled = true;
    }
  }
  const mailboxes = finishNoticeMailboxes.get(input.agentManager);
  for (const [mailboxKey, mailbox] of Array.from(mailboxes ?? [])) {
    if (!input.callerAgentIds.has(mailbox.callerAgentId)) {
      continue;
    }
    if (mailbox.timer) {
      clearTimeout(mailbox.timer);
      mailbox.timer = null;
    }
    mailbox.unsubscribeSettle?.();
    mailbox.unsubscribeSettle = null;
    mailbox.entries.length = 0;
    mailboxes?.delete(mailboxKey);
  }
}

export function setupFinishNotification(params: SetupFinishNotificationParams): void {
  const {
    agentManager,
    agentStorage,
    childAgentId,
    callerAgentId,
    requireParentOwnership = false,
    wake = "always",
    coalesceWindowMs = FINISH_NOTICE_WINDOW_MS,
    logger,
  } = params;
  let hasSeenRunning = false;
  let stopped = false;
  const notifiedPermissionRequestIds = new Set<string>();
  let unsubscribe: (() => void) | null = null;
  let notificationQueue = Promise.resolve();

  const armedByManager =
    armedFinishNotifications.get(agentManager) ?? new Map<string, ArmedFinishNotification>();
  armedFinishNotifications.set(agentManager, armedByManager);
  const armedKey = JSON.stringify([childAgentId, callerAgentId]);
  armedByManager.get(armedKey)?.stop();
  armedByManager.set(armedKey, { callerAgentId, stop });

  function stop(): void {
    if (stopped) return;
    stopped = true;
    unsubscribe?.();
    if (armedByManager.get(armedKey)?.stop === stop) {
      armedByManager.delete(armedKey);
    }
  }

  async function notify(
    reason: FinishNotificationReason,
    delivery: PendingFinishDelivery,
    permissionRequest?: AgentPermissionRequest,
  ): Promise<void> {
    const callerRecord = await agentStorage.get(callerAgentId);
    if (callerRecord?.archivedAt) {
      return;
    }

    const record = await agentStorage.get(childAgentId);
    if (requireParentOwnership && getParentAgentIdFromLabels(record?.labels) !== callerAgentId) {
      return;
    }
    const title = record?.title ?? childAgentId;
    const lastAssistantMessage = await agentManager.getLastAssistantMessage(childAgentId);
    const body = formatFinishNotificationBody({
      childAgentId,
      title,
      reason,
      lastAssistantMessage,
      permissionRequest,
    });

    if (delivery.cancelled) {
      return;
    }
    // A permission request blocks the child, so it skips the coalescing window.
    enqueueFinishNotice({
      agentManager,
      agentStorage,
      logger,
      callerAgentId,
      wake,
      windowMs: coalesceWindowMs,
      entry: {
        key: JSON.stringify([childAgentId, reason, permissionRequest?.id ?? null]),
        childAgentId,
        reason,
        body,
      },
      immediate: reason === "needs permission",
    });
  }

  function notifySafely(reason: FinishNotificationReason, options: NotifySafelyOptions = {}): void {
    if (stopped) return;
    if (options.terminal ?? true) stop();
    const deliveries =
      pendingFinishDeliveries.get(agentManager) ?? new Set<PendingFinishDelivery>();
    pendingFinishDeliveries.set(agentManager, deliveries);
    const delivery: PendingFinishDelivery = { callerAgentId, cancelled: false };
    deliveries.add(delivery);
    notificationQueue = notificationQueue
      .then(() => notify(reason, delivery, options.permissionRequest))
      .catch((error) => {
        logger.error(
          { err: error, childAgentId, callerAgentId, reason },
          "Failed to notify caller agent",
        );
      })
      .finally(() => {
        deliveries.delete(delivery);
      });
  }

  unsubscribe = agentManager.subscribe(
    (event) => {
      if (stopped) {
        return;
      }

      if (event.type === "agent_state") {
        for (const requestId of notifiedPermissionRequestIds) {
          if (!event.agent.pendingPermissions.has(requestId)) {
            notifiedPermissionRequestIds.delete(requestId);
          }
        }
        if (event.agent.lifecycle === "running") {
          if (event.agent.pendingPermissions.size === 0) {
            hasSeenRunning = true;
          }
          return;
        }
        if (event.agent.lifecycle === "error") {
          notifySafely("errored");
          return;
        }
        if (event.agent.lifecycle === "idle" && hasSeenRunning) {
          notifySafely("finished");
          return;
        }
        if (event.agent.lifecycle === "closed") {
          notifySafely("was closed");
          return;
        }
        return;
      }

      if (event.type === "timeline_replacement") {
        return;
      }

      if (event.event.type === "permission_requested") {
        // A permission pause is an intermediate checkpoint. Forget the run
        // observed before it so an idle state during follow-up startup cannot
        // masquerade as the final completion.
        hasSeenRunning = false;
        if (!notifiedPermissionRequestIds.has(event.event.request.id)) {
          notifiedPermissionRequestIds.add(event.event.request.id);
          notifySafely("needs permission", {
            terminal: false,
            permissionRequest: event.event.request,
          });
        }
        return;
      }

      if (event.event.type === "permission_resolved") {
        notifiedPermissionRequestIds.delete(event.event.requestId);
        const childAgent = agentManager.getAgent(childAgentId);
        if (childAgent?.pendingPermissions.size === 0) {
          hasSeenRunning = childAgent.lifecycle === "running";
        }
      }
    },
    { agentId: childAgentId, replayState: false },
  );

  // Check if the child is already running (catches the case where
  // the lifecycle flipped before our subscribe call was processed).
  // Do NOT treat an immediate "idle" as "finished" — the agent may
  // not have started yet (streamAgent sets a pending run before
  // transitioning to "running").
  const childSnapshot = agentManager.getAgent(childAgentId);
  if (!childSnapshot || childSnapshot.lifecycle === "closed") {
    stop();
    return;
  }
  if (childSnapshot.lifecycle === "running") {
    hasSeenRunning = true;
  } else if (childSnapshot.lifecycle === "error") {
    notifySafely("errored");
  }
}
