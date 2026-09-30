import { composerModeKey, type AgentModeState } from "./agent-mode";
import type { PluginClientContext } from "@getpaseo/plugin/client";
import { type TranslateSettings } from "../shared/settings";
import type { Doc, DomEvent, El, Observer } from "./dom";

type Mode = TranslateSettings["cavemanMode"];
type Client = Pick<PluginClientContext, "rpc">;

export const modes: readonly { value: Mode; label: string }[] = [
  { value: "follow-agent", label: "Default" },
  { value: "lite", label: "Lite" },
  { value: "full", label: "Full" },
  { value: "ultra", label: "Ultra" },
  { value: "wenyan-lite", label: "Wenyan Lite" },
  { value: "wenyan-full", label: "Wenyan Full" },
  { value: "wenyan-ultra", label: "Wenyan Ultra" },
];
const ROOT = '[data-testid="message-input-root"]';
const ATTACH = '[data-testid="message-input-attach-button"]';
const MODEL = '[data-testid="combined-model-selector"]';
const CONTROL = "data-prompt-translate-mode";
const OPTION = "data-prompt-translate-option";

const styles = `
[${CONTROL}] { position:relative; display:flex; align-items:center; min-width:0; }
[${CONTROL}] [data-pt-trigger] { appearance:none; display:inline-flex; align-items:center; gap:6px;
  height:28px; max-width:180px; padding:0 8px; border:0; border-radius:999px;
  background:transparent; color:inherit; font:inherit; font-size:14px; white-space:nowrap;
  cursor:pointer; }
[${CONTROL}] [data-pt-trigger]:hover, [${CONTROL}] [data-pt-trigger][aria-expanded="true"] {
  background:color-mix(in srgb,currentColor 9%,transparent); }
[${CONTROL}] [data-pt-trigger]:focus-visible { outline:2px solid currentColor; outline-offset:2px; }
[${CONTROL}] [data-pt-label] { overflow:hidden; text-overflow:ellipsis; }
[${CONTROL}] [data-pt-chevron] { flex:none; width:6px; height:6px; margin-top:-3px;
  border-right:1.5px solid currentColor; border-bottom:1.5px solid currentColor;
  transform:rotate(45deg); opacity:.7; }
[${CONTROL}] [data-pt-menu] { position:absolute; left:0; bottom:calc(100% + 8px); z-index:2147483646;
  min-width:224px; max-height:310px; overflow:auto; padding:6px;
  border:1px solid color-mix(in srgb,var(--pt-foreground) 14%,transparent);
  border-radius:12px; background:var(--pt-surface); color:var(--pt-foreground);
  box-shadow:0 12px 32px rgba(0,0,0,.18); }
[${CONTROL}] [${OPTION}] { appearance:none; display:flex; align-items:center; justify-content:space-between;
  width:100%; min-height:34px; padding:6px 10px; border:0; border-radius:7px;
  background:transparent; color:inherit; font:inherit; font-size:14px; text-align:left;
  cursor:pointer; }
[${CONTROL}] [${OPTION}]:hover { background:color-mix(in srgb,currentColor 9%,transparent); }
[${CONTROL}] [${OPTION}]:focus, [${CONTROL}] [${OPTION}][data-pt-active] {
  background:color-mix(in srgb,currentColor 14%,transparent);
  outline:2px solid currentColor; outline-offset:-2px; }
[${CONTROL}] [data-pt-check] { margin-left:14px; font-size:15px; }
[${CONTROL}] [data-pt-menu]::backdrop {
  background:transparent; pointer-events:auto; -webkit-app-region:no-drag;
}
`;

function label(mode: Mode) {
  return modes.find((item) => item.value === mode)?.label ?? "Default";
}

function surfaceColor(doc: Doc, root: El): { foreground: string; surface: string } {
  const computed = doc.defaultView?.getComputedStyle;
  const foreground = computed?.(root).color || "CanvasText";
  for (let node: El | null = root; node; node = node.parentElement) {
    const background = computed?.(node).backgroundColor;
    if (background && background !== "transparent" && !/^rgba\([^)]*,\s*0\)$/.test(background))
      return { foreground, surface: background };
  }
  return { foreground, surface: "Canvas" };
}

export function installComposerModeMenu(
  _client: Client,
  doc: Doc,
  Observer: Observer | undefined,
  state: AgentModeState,
  owns: (root: El) => boolean = () => true,
) {
  const controls = new Map<El, { trigger: El; label: El; menu: El | null }>();
  const style = doc.createElement("style");
  style.textContent = styles;
  doc.head.append(style);
  let stopped = false;
  let scheduled = false;
  let saving = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function close(wrapper: El) {
    const control = controls.get(wrapper);
    if (!control) return;
    control.menu?.remove();
    control.menu = null;
    control.trigger.setAttribute("aria-expanded", "false");
  }

  async function save(mode: Mode, trigger: El) {
    const root = trigger.closest(ROOT);
    if (saving || !root || !owns(root)) return;
    saving = true;
    trigger.setAttribute("aria-busy", "true");
    const id = composerModeKey(root);
    try {
      if (!id) throw new Error("Open an agent conversation first");
      await state.set(id, mode);
      update();
      trigger.removeAttribute("title");
    } catch (error) {
      trigger.setAttribute("title", error instanceof Error ? error.message : String(error));
    } finally {
      saving = false;
      trigger.removeAttribute("aria-busy");
    }
  }

  function open(wrapper: El) {
    const control = controls.get(wrapper);
    const root = wrapper.closest(ROOT);
    if (!control || control.menu || !root || !owns(root)) return;
    for (const other of controls.keys()) close(other);
    const id = composerModeKey(root);
    if (!id || state.get(id) === undefined) return;
    const selectedMode = state.get(id);
    const choose = (mode: Mode) => {
      close(wrapper);
      control.trigger.focus?.();
      void save(mode, control.trigger);
    };
    const menu = doc.createElement("div");
    menu.setAttribute("data-pt-menu", "");
    menu.setAttribute("role", "listbox");
    menu.setAttribute("aria-label", "Caveman mode");
    const colors = surfaceColor(doc, wrapper);
    menu.style.cssText = `--pt-surface:${colors.surface};--pt-foreground:${colors.foreground};`;
    // The draft toolbar clips overflow. A popover escapes it through the top layer.
    const anchor = control.trigger.getBoundingClientRect?.();
    const height = doc.defaultView?.innerHeight;
    if (menu.showPopover && anchor && height) {
      menu.setAttribute("popover", "auto");
      menu.style.cssText += `;position:fixed;margin:0;left:${Math.max(8, Math.min(anchor.left, (doc.defaultView?.innerWidth ?? 1024) - 246))}px;top:auto;bottom:${height - anchor.top + 8}px;max-height:${Math.max(34, Math.min(310, anchor.top - 24))}px;`;
    }
    for (const mode of modes) {
      const option = doc.createElement("button");
      option.setAttribute("type", "button");
      option.setAttribute(OPTION, mode.value);
      option.setAttribute("role", "option");
      option.setAttribute("aria-selected", String(mode.value === selectedMode));
      const name = doc.createElement("span");
      name.textContent = mode.label;
      option.append(name);
      if (mode.value === selectedMode) {
        const check = doc.createElement("span");
        check.setAttribute("data-pt-check", "");
        check.setAttribute("aria-hidden", "true");
        check.textContent = "✓";
        option.append(check);
      }
      option.setAttribute("tabindex", "-1");
      option.addEventListener("click", () => choose(mode.value));
      option.addEventListener("focus", () => {
        for (const other of Array.from(menu.querySelectorAll(`[${OPTION}]`)))
          other.removeAttribute("data-pt-active");
        option.setAttribute("data-pt-active", "");
        menu.setAttribute("aria-activedescendant", option.getAttribute("id") ?? "");
      });
      option.setAttribute("id", `pt-option-${mode.value}`);
      menu.append(option);
    }
    control.menu = menu;
    // Native light-dismiss must also clear our state so the trigger can reopen it.
    menu.addEventListener("toggle", (event) => {
      if (event.newState === "closed" && control.menu === menu) close(wrapper);
    });
    wrapper.append(menu);
    if (menu.hasAttribute("popover")) menu.showPopover?.();
    control.trigger.setAttribute("aria-expanded", "true");
  }

  // Outermost node wrapping only the model selector, so the menu sits beside it
  // rather than inside its fixed-width viewport.
  function modelSlot(root: El) {
    let slot = root.querySelector(MODEL);
    while (
      slot?.parentElement &&
      slot.parentElement !== root &&
      slot.parentElement.childElementCount === 1
    )
      slot = slot.parentElement;
    return slot;
  }

  function scan() {
    scheduled = false;
    if (stopped) return;
    for (const wrapper of controls.keys()) {
      const root = wrapper.closest(ROOT);
      if (!wrapper.isConnected || !root || !owns(root)) {
        close(wrapper);
        wrapper.remove();
        controls.delete(wrapper);
      }
    }
    for (const root of Array.from(doc.querySelectorAll(ROOT))) {
      if (!owns(root)) continue;
      const id = composerModeKey(root);
      if (id) void state.load(id).catch(() => undefined);
      const slot = modelSlot(root);
      const existing = root.querySelector(`[${CONTROL}]`);
      if (existing) {
        // React can mount or reorder toolbar children after the menu is inserted.
        if (slot && existing.nextElementSibling !== slot) slot.before(existing);
        continue;
      }
      const attach = root.querySelector(ATTACH);
      const agentControl = root.querySelector(
        '[data-testid="agent-provider-selector"], [data-testid="mode-control"]',
      );
      let group = attach?.parentElement;
      while (group && agentControl && !group.contains(agentControl)) group = group.parentElement;
      if (!group && !slot) continue;
      const wrapper = doc.createElement("div");
      wrapper.setAttribute(CONTROL, "");
      const trigger = doc.createElement("button");
      trigger.setAttribute("type", "button");
      trigger.setAttribute("data-pt-trigger", "");
      trigger.setAttribute("aria-label", "Caveman mode");
      trigger.setAttribute("aria-haspopup", "listbox");
      trigger.setAttribute("aria-expanded", "false");
      const name = doc.createElement("span");
      name.setAttribute("data-pt-label", "");
      name.textContent = "Caveman: Default";
      const chevron = doc.createElement("span");
      chevron.setAttribute("data-pt-chevron", "");
      chevron.setAttribute("aria-hidden", "true");
      trigger.append(name, chevron);
      trigger.addEventListener("click", () => {
        if (controls.get(wrapper)?.menu) close(wrapper);
        else open(wrapper);
      });
      const focusOption = (index: number) => {
        const options = Array.from(wrapper.querySelectorAll(`[${OPTION}]`));
        if (!options.length) return;
        options[((index % options.length) + options.length) % options.length]?.focus?.();
      };
      const focusedIndex = (target: unknown) => {
        const options = Array.from(wrapper.querySelectorAll(`[${OPTION}]`));
        const option = (target as El | null)?.closest?.(`[${OPTION}]`) ?? null;
        return option ? options.indexOf(option) : -1;
      };
      const selectedIndex = () =>
        Array.from(wrapper.querySelectorAll(`[${OPTION}]`)).findIndex(
          (option) => option.getAttribute("aria-selected") === "true",
        );
      wrapper.addEventListener("keydown", (event) => {
        const isOpen = Boolean(controls.get(wrapper)?.menu);
        const index = focusedIndex(event.target);
        const onOption = index >= 0;
        const key = event.key;
        const stop = () => {
          event.preventDefault();
          event.stopPropagation();
        };
        if (key === "Escape") {
          if (!isOpen) return;
          stop();
          close(wrapper);
          trigger.focus?.();
        } else if (key === "Tab") {
          if (isOpen) close(wrapper);
        } else if (key === "ArrowDown" || key === "ArrowUp") {
          stop();
          if (!isOpen) {
            open(wrapper);
            const selected = selectedIndex();
            focusOption(selected >= 0 ? selected : key === "ArrowDown" ? 0 : -1);
            return;
          }
          if (!onOption) {
            const selected = selectedIndex();
            focusOption(selected >= 0 ? selected : key === "ArrowDown" ? 0 : -1);
          } else focusOption(key === "ArrowDown" ? index + 1 : index - 1);
        } else if (key === "Home" || key === "End") {
          if (!isOpen) return;
          stop();
          focusOption(key === "Home" ? 0 : -1);
        } else if (key === "Enter" || key === " ") {
          if (onOption) {
            stop();
            const option = (event.target as El).closest(`[${OPTION}]`)!;
            const mode = option.getAttribute(OPTION) as Mode;
            close(wrapper);
            trigger.focus?.();
            void save(mode, trigger);
          } else if (!isOpen && trigger.getAttribute("disabled") === null) {
            stop();
            open(wrapper);
            const selected = selectedIndex();
            focusOption(selected >= 0 ? selected : 0);
          }
        }
      });
      wrapper.append(trigger);
      controls.set(wrapper, { trigger, label: name, menu: null });
      if (slot) slot.before(wrapper);
      else if (attach && agentControl && group) {
        let anchor = attach;
        while (anchor.parentElement && anchor.parentElement !== group)
          anchor = anchor.parentElement;
        anchor.after(wrapper);
      } else group?.append(wrapper);
    }
  }

  function update() {
    for (const [wrapper, control] of controls) {
      const root = wrapper.closest(ROOT);
      if (!root || !owns(root)) {
        close(wrapper);
        wrapper.remove();
        controls.delete(wrapper);
        continue;
      }
      const model = root.querySelector('[data-testid="combined-model-selector"]');
      const modelLabel =
        model &&
        Array.from(model.querySelectorAll("div,span")).find(
          (node) => node.textContent?.trim() && !node.querySelector("div,span,svg"),
        );
      const typography = modelLabel && doc.defaultView?.getComputedStyle?.(modelLabel);
      if (typography) {
        const css = `color:${typography.color};font-family:${typography.fontFamily};font-size:${typography.fontSize};font-weight:${typography.fontWeight};font-style:${typography.fontStyle};line-height:${typography.lineHeight};letter-spacing:${typography.letterSpacing};`;
        if (control.trigger.getAttribute("data-pt-typography") !== css) {
          control.trigger.style.cssText = css;
          control.trigger.setAttribute("data-pt-typography", css);
        }
      }
      const id = composerModeKey(root);
      const mode = id ? state.get(id) : undefined;
      const text = `Caveman: ${mode ? label(mode) : id ? "Loading…" : "Default"}`;
      if (control.label.textContent !== text) {
        control.label.textContent = text;
        close(wrapper);
      }
      if (id && mode) control.trigger.removeAttribute("disabled");
      else control.trigger.setAttribute("disabled", "");
    }
  }
  const unsubscribe = state.subscribe(update);
  // The first native hook can initialize the agent after its composer mounts.
  const refresh = setInterval(() => {
    for (const wrapper of controls.keys()) {
      if (!wrapper.isConnected) continue;
      const root = wrapper.closest(ROOT);
      const id = root && owns(root) && composerModeKey(root);
      if (id && !id.startsWith("draft:")) void state.load(id, true).catch(() => undefined);
    }
  }, 2000);
  function outside(event: DomEvent) {
    const target = event.target as El | null;
    for (const wrapper of controls.keys()) if (!wrapper.contains(target)) close(wrapper);
  }
  doc.body.addEventListener("pointerdown", outside, true);
  const observer = Observer
    ? new Observer(() => {
        if (scheduled || stopped) return;
        scheduled = true;
        timer = setTimeout(() => {
          scan();
          update();
        }, 50);
      })
    : undefined;
  observer?.observe(doc.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["class", "style"],
  });
  scan();
  update();

  return {
    scan,
    update,
    stop() {
      stopped = true;
      unsubscribe();
      clearInterval(refresh);
      observer?.disconnect();
      clearTimeout(timer);
      doc.body.removeEventListener("pointerdown", outside, true);
      for (const wrapper of controls.keys()) wrapper.remove();
      controls.clear();
      style.remove();
    },
  };
}
