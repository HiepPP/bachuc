import { reactProps, type Doc, type El, type Key } from "./dom";
import { composerAgent, composerModeKey, type Mode } from "./agent-mode";
import { preserveCavemanCommand } from "../shared/caveman";
import type { TranslateSettings } from "../shared/settings";
import type { DomEvent } from "./dom";

export type ComposerApi = {
  enhance(text: string): Promise<string>;
  mode(agentId: string): Promise<Mode>;
  prepare(input: {
    agentId: string;
    text: string;
    source: string;
    mode: Mode;
  }): Promise<{ token: string }>;
  bindQueue(agentId: string, token: string, queueId: string): Promise<unknown>;
  cancelQueue(agentId: string, queueId: string): Promise<unknown>;
  cancel(agentId: string, token: string): Promise<unknown>;
};

// The host marks the field with dataSet={{composerInput:""}}; the root testID covers older hosts.
const FIELD = '[data-composer-input], [data-testid="message-input-root"] textarea';
const BADGE = "data-prompt-translate-badge";
const UNSENT = "Chưa gửi được, nhấn Enter để gửi";

// The field is uncontrolled, but React tracks the last value it saw: write through the prototype
// setter and replay the input event so the host adopts the new text.
export function fillField(field: El, text: string, doc: Doc, focus = true): boolean {
  const view = doc.defaultView;
  if (!view) return false;
  const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), "value")?.set;
  if (setter) setter.call(field, text);
  else field.value = text;
  field.dispatchEvent(new view.Event("input", { bubbles: true }));
  if (focus) {
    field.focus?.();
    field.setSelectionRange?.(text.length, text.length);
  }
  return field.value === text;
}

// A synthetic plain Enter runs the host's default send action, which honors the user's
// send-or-queue preference while the agent runs. Compact windows do not submit on Enter.
function pressEnter(field: El, doc: Doc, modifiers: Partial<Key> = {}): boolean {
  const Keyboard = doc.defaultView?.KeyboardEvent;
  if (!Keyboard) return false;
  field.dispatchEvent(
    new Keyboard("keydown", {
      key: "Enter",
      code: "Enter",
      bubbles: true,
      cancelable: true,
      metaKey: modifiers.metaKey,
      ctrlKey: modifiers.ctrlKey,
    }),
  );
  return true;
}

export function installComposer(
  api: ComposerApi,
  options: { enabled(): boolean; settings(): TranslateSettings; owns?(field: El): boolean },
  doc: Doc,
  // send: let React adopt the new text before Enter; verify: when an unsent prompt means failure.
  timing = { send: 50, verify: 1_500 },
) {
  const owns = options.owns ?? (() => true);
  const firstTurnText = new WeakMap<El, { prompt: string; original: string }>();
  const queueBindings = new Map<string, Promise<unknown>>();
  let editingQueue = false;
  let replayingQueueEdit = false;
  function queueRows() {
    const rows = new Map<string, { id: string; text: string; button: El; editLabel: string }>();
    for (const button of Array.from(doc.querySelectorAll('button,[role="button"]'))) {
      const props = reactProps(
        button,
        (p) =>
          typeof p.onEdit === "function" &&
          typeof p.onSendNow === "function" &&
          typeof p.editLabel === "string",
      );
      const item = props?.item as { id?: string; text?: string } | undefined;
      if (typeof item?.id === "string" && typeof item.text === "string")
        rows.set(item.id, {
          id: item.id,
          text: item.text,
          button,
          editLabel: props!.editLabel as string,
        });
    }
    return rows;
  }
  let pending: { field: El; draft: string } | null = null;
  let syntheticSend: El | null = null;
  let badge: El | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  const timers = new Set<ReturnType<typeof setTimeout>>();
  const later = (ms: number, run: () => void) => {
    const timer = setTimeout(() => {
      timers.delete(timer);
      run();
    }, ms);
    timers.add(timer);
  };

  async function prepareSend(field: El, source: string, replay: () => void) {
    const draft = field.value ?? "";
    const agentId = composerAgent(field);
    if (!owns(field) || !draft.trim() || pending || editingQueue) return;
    if (!agentId) {
      const key = composerModeKey(field);
      const request = { field, draft };
      pending = request;
      try {
        const mode = key ? await api.mode(key) : "follow-agent";
        if (
          pending !== request ||
          !owns(field) ||
          !field.isConnected ||
          field.value !== draft ||
          composerModeKey(field) !== key
        )
          return;
        const previous = firstTurnText.get(field);
        const original = previous?.prompt === draft ? previous.original : draft;
        const explicit = preserveCavemanCommand("", original);
        const prompt =
          explicit || mode === "follow-agent" ? original : `/caveman ${mode}\n\n${original}`;
        firstTurnText.set(field, { prompt, original });
        if (prompt !== draft) {
          if (!fillField(field, prompt, doc)) return;
          await new Promise<void>((resolve) => setTimeout(resolve, timing.send));
        }
        if (
          pending !== request ||
          !owns(field) ||
          !field.isConnected ||
          field.value !== prompt ||
          composerModeKey(field) !== key
        )
          return;
        const latest = key ? await api.mode(key) : mode;
        if (
          !owns(field) ||
          pending !== request ||
          !field.isConnected ||
          field.value !== prompt ||
          composerModeKey(field) !== key
        )
          return;
        if (latest !== mode) {
          fillField(field, original, doc);
          pending = null;
          return prepareSend(field, source, replay);
        }
        syntheticSend = field;
        try {
          replay();
        } finally {
          syntheticSend = null;
        }
      } catch (error) {
        say(`Caveman: ${String(error)}`, 4000);
      } finally {
        if (pending === request) pending = null;
      }
      return;
    }
    const request = { field, draft };
    pending = request;
    const valid = () =>
      pending === request &&
      owns(field) &&
      field.isConnected &&
      field.value === draft &&
      composerAgent(field) === agentId;
    let token: string | undefined;
    try {
      let mode = await api.mode(agentId);
      while (valid()) {
        token = (await api.prepare({ agentId, text: draft, source, mode })).token;
        const latest = await api.mode(agentId);
        if (!valid() || latest !== mode) {
          await api.cancel(agentId, token);
          token = undefined;
          mode = latest;
          continue;
        }
        const existingQueue = queueRows();
        pending = null;
        syntheticSend = field;
        try {
          replay();
        } finally {
          syntheticSend = null;
        }
        const sentToken = token;
        token = undefined;
        // React commits the queue row after the send event. Bind its ID, not its text:
        // identical prompts can be queued more than once with different modes.
        later(0, () => {
          const row = Array.from(queueRows().values()).find(
            (row) => !existingQueue.has(row.id) && row.text === draft.trim(),
          );
          if (row) {
            const binding = api.bindQueue(agentId, sentToken, row.id);
            queueBindings.set(row.id, binding);
            void binding.catch((error) => say(`Caveman queue: ${String(error)}`, 4000));
          }
        });
        later(timing.verify, () => {
          if (
            owns(field) &&
            composerAgent(field) === agentId &&
            field.isConnected &&
            field.value === draft
          ) {
            void api.cancel(agentId, sentToken).catch(() => undefined);
            say(UNSENT, 4000);
          }
        });
        return;
      }
    } catch (error) {
      say(`Caveman hook: ${error instanceof Error ? error.message : String(error)}`, 4000);
    } finally {
      if (token) void api.cancel(agentId, token).catch(() => undefined);
      if (pending === request) pending = null;
    }
  }
  function send(field: El, prompt: string, source: string) {
    const agentId = composerModeKey(field);
    later(timing.send, () => {
      if (
        owns(field) &&
        field.isConnected &&
        field.value === prompt &&
        composerModeKey(field) === agentId
      )
        void prepareSend(field, source, () => {
          if (!pressEnter(field, doc)) say(UNSENT, 4000);
        });
    });
  }

  function say(text: string | null, ms?: number) {
    clearTimeout(hideTimer);
    if (text === null) {
      badge?.remove();
      badge = null;
      return;
    }
    if (!badge) {
      badge = doc.createElement("div");
      badge.setAttribute(BADGE, "");
      // Fixed position keeps the host layout untouched.
      badge.style.cssText =
        "position:fixed;right:16px;bottom:16px;z-index:2147483647;padding:6px 10px;border-radius:8px;font:12px/1.4 system-ui,sans-serif;background:rgba(24,24,27,.9);color:#fafafa;pointer-events:none;";
      doc.body.append(badge);
    }
    badge.textContent = text;
    if (ms) hideTimer = setTimeout(() => say(null), ms);
  }

  function onKeydown(event: Key) {
    const field = (event.target as El | null)?.closest?.(FIELD) ?? null;
    if (!field || !owns(field)) return;
    if (event.key === "Escape" && pending) {
      pending = null;
      say(null);
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }
    if (event.key !== "Enter" && event.key !== " ") return;
    if (event.shiftKey) return;
    if (event.isComposing || event.keyCode === 229) return;
    if (event.key !== "Enter") return;
    if (syntheticSend === field) return;
    // In compact windows Enter inserts a newline; an open autocomplete consumes Enter.
    if ((doc.defaultView?.innerWidth ?? 720) < 720) return;
    if (doc.querySelectorAll('[data-testid="composer-autocomplete-popover"]').length) return;
    if (!(event.metaKey || event.ctrlKey) || !options.enabled()) {
      event.preventDefault();
      event.stopImmediatePropagation();
      void prepareSend(field, field.value ?? "", () => pressEnter(field, doc, event));
      return;
    }
    // Owned even when idle or empty, so the host never sends or queues on Cmd+Enter.
    event.preventDefault();
    event.stopImmediatePropagation();
    const draft = field.value ?? "";
    if (pending || !draft.trim()) return;
    const request = { field, draft, agentId: composerModeKey(field) };
    pending = request;
    say("Enhancing… (Esc để hủy)");
    api.enhance(draft).then(
      (prompt) => {
        if (pending !== request) return;
        pending = null;
        if (!owns(field) || !field.isConnected || composerModeKey(field) !== request.agentId)
          return say(null);
        if (field.value !== draft) return say("Bản nháp đã đổi, bỏ qua kết quả enhance", 4000);
        if (!fillField(field, prompt, doc)) return say("Composer không nhận text đã enhance", 4000);
        say(null);
        send(field, prompt, draft);
      },
      (error: unknown) => {
        if (pending !== request) return;
        pending = null;
        say(`Enhance lỗi: ${error instanceof Error ? error.message : String(error)}`, 4000);
      },
    );
  }

  function onClick(event: DomEvent) {
    const button = (event.target as El | null)?.closest?.('button,[role="button"]');
    if (button && !replayingQueueEdit) {
      const row = Array.from(queueRows().values()).find(
        (row) =>
          row.button === button ||
          reactProps(button, (p) => (p.item as { id?: string } | undefined)?.id === row.id),
      );
      if (row && button.getAttribute("aria-label") === row.editLabel) {
        let parent: El | null = button;
        while (parent && !parent.querySelector(FIELD)) parent = parent.parentElement;
        const field = parent?.querySelector(FIELD);
        const agentId = field && composerAgent(field);
        if (field && owns(field) && agentId) {
          event.preventDefault();
          event.stopImmediatePropagation();
          if (editingQueue) return;
          editingQueue = true;
          void (async () => {
            try {
              await queueBindings.get(row.id);
              await api.cancelQueue(agentId, row.id);
              if (!owns(field) || !field.isConnected || composerAgent(field) !== agentId) return;
              replayingQueueEdit = true;
              try {
                button.click();
              } finally {
                replayingQueueEdit = false;
              }
              queueBindings.delete(row.id);
            } catch (error) {
              say(`Caveman queue: ${error instanceof Error ? error.message : String(error)}`, 4000);
            } finally {
              editingQueue = false;
            }
          })();
          return;
        }
      }
    }
    const root = button?.closest?.('[data-testid="message-input-root"]');
    if (!button || !root || button.closest?.("[data-prompt-translate-mode]")) return;
    const buttons = Array.from(root.querySelectorAll('button,[role="button"]'));
    if (buttons.at(-1) !== button) return;
    const field = root.querySelector(FIELD);
    if (!field || !owns(field) || syntheticSend === field || !field.value?.trim()) return;
    const action = reactProps(button, (props) => typeof props.onDefaultSendAction === "function");
    if (action?.canPressLoadingButton) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    void prepareSend(field, field.value ?? "", () => button.click());
  }

  doc.addEventListener("keydown", onKeydown, true);
  doc.addEventListener("click", onClick, true);
  return {
    onKeydown,
    onClick,
    stop() {
      doc.removeEventListener("keydown", onKeydown, true);
      doc.removeEventListener("click", onClick, true);
      for (const timer of timers) clearTimeout(timer);
      timers.clear();
      pending = null;
      say(null);
    },
  };
}
