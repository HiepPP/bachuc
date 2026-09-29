import { catalog, type SkillId } from "../shared/catalog";
import { ROOT, composerAgent, type Doc, type DomEvent, type El, type Observer } from "./dom";
import { DRAFT, type SkillState } from "./state";

const MODEL = '[data-testid="combined-model-selector"]';
const CAVEMAN = "[data-prompt-translate-mode]";
const CONTROL = "data-skill-pins";
const OPTION = "data-sp-option";

// Same rules as prompt-translate's Caveman menu (client/mode-menu.ts) so both pills match.
const styles = `
[${CONTROL}] { position:relative; display:flex; align-items:center; min-width:0; }
[${CONTROL}] [data-sp-trigger] { appearance:none; display:inline-flex; align-items:center; gap:6px;
  height:28px; max-width:180px; padding:0 8px; border:0; border-radius:999px;
  background:transparent; color:inherit; font:inherit; font-size:14px; white-space:nowrap;
  cursor:pointer; }
[${CONTROL}] [data-sp-trigger]:hover, [${CONTROL}] [data-sp-trigger][aria-expanded="true"] {
  background:color-mix(in srgb,currentColor 9%,transparent); }
[${CONTROL}] [data-sp-trigger]:focus-visible { outline:2px solid currentColor; outline-offset:2px; }
[${CONTROL}] [data-sp-label] { overflow:hidden; text-overflow:ellipsis; }
[${CONTROL}] [data-sp-sep] { opacity:.55; }
[${CONTROL}][data-sp-state="empty"] [data-sp-trigger] { opacity:.7; }
[${CONTROL}][data-sp-state="active"] [data-sp-trigger] {
  background:color-mix(in srgb,currentColor 12%,transparent); }
[${CONTROL}][data-sp-state="active"] [data-sp-trigger]:hover,
[${CONTROL}][data-sp-state="active"] [data-sp-trigger][aria-expanded="true"] {
  background:color-mix(in srgb,currentColor 16%,transparent); }
[${CONTROL}] [data-sp-trigger]:disabled { opacity:.4; cursor:default; background:transparent; }
[${CONTROL}] [data-sp-chevron] { flex:none; width:6px; height:6px; margin-top:-3px;
  border-right:1.5px solid currentColor; border-bottom:1.5px solid currentColor;
  transform:rotate(45deg); opacity:.7; }
[${CONTROL}] [data-sp-menu] { position:absolute; left:0; bottom:calc(100% + 8px); z-index:2147483646;
  width:300px; max-height:360px; overflow:auto; padding:6px;
  border:1px solid color-mix(in srgb,var(--sp-foreground) 14%,transparent);
  border-radius:12px; background:var(--sp-surface); color:var(--sp-foreground);
  box-shadow:0 12px 32px rgba(0,0,0,.18); }
[${CONTROL}] [data-sp-head] { padding:6px 10px 4px; font-size:11px; font-weight:600;
  letter-spacing:.02em; text-transform:uppercase; opacity:.65; }
[${CONTROL}] [${OPTION}], [${CONTROL}] [data-sp-clear] { appearance:none; display:flex;
  align-items:flex-start; justify-content:space-between; gap:12px;
  width:100%; min-height:34px; padding:7px 10px; border:0; border-radius:7px;
  background:transparent; color:inherit; font:inherit; font-size:14px; text-align:left;
  cursor:pointer; }
[${CONTROL}] [${OPTION}]:hover, [${CONTROL}] [data-sp-clear]:not(:disabled):hover {
  background:color-mix(in srgb,currentColor 9%,transparent); }
[${CONTROL}] [${OPTION}]:focus-visible, [${CONTROL}] [data-sp-clear]:focus-visible {
  background:color-mix(in srgb,currentColor 14%,transparent);
  outline:2px solid currentColor; outline-offset:-2px; }
[${CONTROL}] [data-sp-desc] { display:block; font-size:12px; opacity:.65; margin-top:1px; }
[${CONTROL}] [data-sp-warn] { display:block; font-size:12px; margin-top:2px;
  color:color-mix(in srgb,#d98e04 75%,currentColor); }
[${CONTROL}] [data-sp-check] { flex:none; width:14px; margin-top:1px; font-size:15px; text-align:right; }
[${CONTROL}] [data-sp-divider] { border-top:1px solid color-mix(in srgb,currentColor 12%,transparent);
  margin:6px 4px; }
[${CONTROL}] [data-sp-clear] { opacity:.7; }
[${CONTROL}] [data-sp-clear]:disabled { opacity:.35; cursor:default; background:transparent; }
[${CONTROL}] [data-sp-menu]::backdrop {
  background:transparent; pointer-events:auto; -webkit-app-region:no-drag;
}
`;

const pinned = (skills: readonly SkillId[] | undefined) =>
  catalog.filter((skill) => skills?.includes(skill.id));

export function pillLabel(skills: readonly SkillId[] | undefined) {
  const chosen = pinned(skills);
  return chosen.length ? chosen.map((skill) => skill.short).join(" · ") : "Skills";
}

export function pillTitle(skills: readonly SkillId[] | undefined) {
  const chosen = pinned(skills);
  return chosen.length
    ? `Pinned: ${chosen.map((skill) => skill.label).join(", ")}`
    : "Pin skills for this chat";
}

function surfaceColor(doc: Doc, root: El) {
  const computed = doc.defaultView?.getComputedStyle;
  const foreground = computed?.(root).color || "CanvasText";
  for (let node: El | null = root; node; node = node.parentElement) {
    const background = computed?.(node).backgroundColor;
    if (background && background !== "transparent" && !/^rgba\([^)]*,\s*0\)$/.test(background))
      return { foreground, surface: background };
  }
  return { foreground, surface: "Canvas" };
}

// Outermost node wrapping only the model selector, as in prompt-translate.
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

const stateKey = (root: El) => composerAgent(root) ?? DRAFT;

export function installSkillsMenu(
  doc: Doc,
  Observer: Observer | undefined,
  state: SkillState,
  owns: (root: El) => boolean = () => true,
) {
  const controls = new Map<El, { trigger: El; label: El; menu: El | null }>();
  const style = doc.createElement("style");
  style.textContent = styles;
  doc.head.append(style);
  let stopped = false;
  let scheduled = false;
  let hadDraft = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function close(wrapper: El) {
    const control = controls.get(wrapper);
    if (!control?.menu) return;
    control.menu.remove();
    control.menu = null;
    control.trigger.setAttribute("aria-expanded", "false");
  }

  function paintMenu(menu: El | null, skills: readonly SkillId[]) {
    if (!menu) return;
    for (const option of Array.from(menu.querySelectorAll(`[${OPTION}]`))) {
      const checked = skills.includes(option.getAttribute(OPTION) as SkillId);
      option.setAttribute("aria-selected", String(checked));
      const check = option.querySelector("[data-sp-check]");
      if (check) check.textContent = checked ? "✓" : "";
    }
    // The clear row always renders, so the bottom-anchored menu keeps its height; it is only
    // disabled while nothing is pinned. `disabled` is not an observed attribute, so repainting
    // cannot retrigger the scan.
    const clear = menu.querySelector("[data-sp-clear]");
    if (skills.length) clear?.removeAttribute("disabled");
    else clear?.setAttribute("disabled", "");
  }

  async function save(wrapper: El, next: (current: SkillId[]) => SkillId[]) {
    const root = wrapper.closest(ROOT);
    const agentId = root && owns(root) ? stateKey(root) : null;
    const control = controls.get(wrapper);
    if (!agentId || !control) return;
    try {
      await state.set(agentId, next(state.get(agentId) ?? []));
    } catch (error) {
      control.trigger.setAttribute("title", error instanceof Error ? error.message : String(error));
    }
    paintMenu(control.menu, state.get(agentId) ?? []);
  }

  function open(wrapper: El) {
    const control = controls.get(wrapper);
    const root = wrapper.closest(ROOT);
    if (!control || control.menu || !root || !owns(root)) return;
    const selected = state.get(stateKey(root));
    if (!selected) return;
    for (const other of controls.keys()) close(other);
    const menu = doc.createElement("div");
    menu.setAttribute("data-sp-menu", "");
    menu.setAttribute("role", "listbox");
    menu.setAttribute("aria-multiselectable", "true");
    menu.setAttribute("aria-label", "Pinned skills");
    const colors = surfaceColor(doc, wrapper);
    menu.style.cssText = `--sp-surface:${colors.surface};--sp-foreground:${colors.foreground};`;
    // The draft toolbar clips overflow. A popover escapes it through the top layer.
    const anchor = control.trigger.getBoundingClientRect?.();
    const height = doc.defaultView?.innerHeight;
    if (menu.showPopover && anchor && height) {
      menu.setAttribute("popover", "auto");
      menu.style.cssText += `;position:fixed;margin:0;left:${Math.max(8, Math.min(anchor.left, (doc.defaultView?.innerWidth ?? 1024) - 308))}px;top:auto;bottom:${height - anchor.top + 8}px;`;
    }
    const head = doc.createElement("div");
    head.setAttribute("data-sp-head", "");
    head.textContent = "Pinned for this chat";
    menu.append(head);
    for (const skill of catalog) {
      const option = doc.createElement("button");
      option.setAttribute("type", "button");
      option.setAttribute("role", "option");
      option.setAttribute("tabindex", "-1");
      option.setAttribute(OPTION, skill.id);
      const text = doc.createElement("span");
      const name = doc.createElement("span");
      name.textContent = skill.label;
      const description = doc.createElement("span");
      description.setAttribute("data-sp-desc", "");
      description.textContent = skill.description;
      text.append(name, description);
      if ("warn" in skill) {
        const warn = doc.createElement("span");
        warn.setAttribute("data-sp-warn", "");
        warn.textContent = skill.warn;
        text.append(warn);
      }
      const check = doc.createElement("span");
      check.setAttribute("data-sp-check", "");
      check.setAttribute("aria-hidden", "true");
      option.append(text, check);
      // Toggling keeps the menu open so several skills can be picked in a row.
      option.addEventListener(
        "click",
        () =>
          void save(wrapper, (current) =>
            current.includes(skill.id)
              ? current.filter((id) => id !== skill.id)
              : [...current, skill.id],
          ),
      );
      menu.append(option);
    }
    const divider = doc.createElement("div");
    divider.setAttribute("data-sp-divider", "");
    const clear = doc.createElement("button");
    clear.setAttribute("type", "button");
    clear.setAttribute("tabindex", "-1");
    clear.setAttribute("data-sp-clear", "");
    clear.textContent = "Bỏ chọn tất cả";
    clear.addEventListener("click", () => void save(wrapper, () => []));
    menu.append(divider, clear);
    paintMenu(menu, selected);
    control.menu = menu;
    menu.addEventListener("toggle", (event) => {
      if (event.newState === "closed" && control.menu === menu) close(wrapper);
    });
    wrapper.append(menu);
    if (menu.hasAttribute("popover")) menu.showPopover?.();
    control.trigger.setAttribute("aria-expanded", "true");
  }

  function focusOption(wrapper: El, target: unknown, step: number) {
    const selector = `[${OPTION}], [data-sp-clear]:not([disabled])`;
    const options = Array.from(wrapper.querySelectorAll(selector));
    if (!options.length) return;
    const current = (target as El | null)?.closest?.(selector) ?? null;
    const index = current ? options.indexOf(current) + step : step > 0 ? 0 : -1;
    options[((index % options.length) + options.length) % options.length]?.focus?.();
  }

  function create(root: El) {
    const wrapper = doc.createElement("div");
    wrapper.setAttribute(CONTROL, "");
    const trigger = doc.createElement("button");
    trigger.setAttribute("type", "button");
    trigger.setAttribute("data-sp-trigger", "");
    trigger.setAttribute("aria-label", "Pinned skills");
    trigger.setAttribute("aria-haspopup", "listbox");
    trigger.setAttribute("aria-expanded", "false");
    trigger.setAttribute("disabled", "");
    const label = doc.createElement("span");
    label.setAttribute("data-sp-label", "");
    label.textContent = "Skills";
    const chevron = doc.createElement("span");
    chevron.setAttribute("data-sp-chevron", "");
    chevron.setAttribute("aria-hidden", "true");
    trigger.append(label, chevron);
    trigger.addEventListener("click", () => {
      if (controls.get(wrapper)?.menu) close(wrapper);
      else open(wrapper);
    });
    wrapper.addEventListener("keydown", (event) => {
      const isOpen = Boolean(controls.get(wrapper)?.menu);
      if (!isOpen) return;
      const stop = () => {
        event.preventDefault();
        event.stopPropagation();
      };
      if (event.key === "Escape") {
        stop();
        close(wrapper);
        trigger.focus?.();
      } else if (event.key === "Tab") close(wrapper);
      else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        stop();
        focusOption(wrapper, event.target, event.key === "ArrowDown" ? 1 : -1);
      }
    });
    wrapper.append(trigger);
    controls.set(wrapper, { trigger, label, menu: null });
    place(root, wrapper);
  }

  // Sit before the Caveman pill, or before the model selector without it. Keeping the
  // Caveman pill's next sibling unchanged avoids a reorder loop with prompt-translate.
  function place(root: El, wrapper: El) {
    const target = root.querySelector(CAVEMAN) ?? modelSlot(root);
    if (target && wrapper.nextElementSibling !== target) target.before(wrapper);
    else if (!target && !wrapper.isConnected) root.append(wrapper);
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
    let draftShown = false;
    for (const root of Array.from(doc.querySelectorAll(ROOT))) {
      if (!owns(root)) continue;
      const key = stateKey(root);
      draftShown ||= key === DRAFT;
      // A draft is consumed by the next created agent, so reread it each time a new-thread
      // composer appears.
      if (key !== DRAFT || !hadDraft) void state.load(key, key === DRAFT).catch(() => undefined);
      const existing = root.querySelector(`[${CONTROL}]`);
      if (existing) place(root, existing);
      else if (root.querySelector(MODEL) || root.querySelector(CAVEMAN)) create(root);
    }
    hadDraft = draftShown;
  }

  function update() {
    for (const [wrapper, control] of controls) {
      const root = wrapper.closest(ROOT);
      if (!root || !owns(root)) continue;
      // Copy the model selector's text style, as the Caveman pill does, so both match the host.
      const model = root.querySelector(MODEL);
      const modelLabel =
        model &&
        Array.from(model.querySelectorAll("div,span")).find(
          (node) => node.textContent?.trim() && !node.querySelector("div,span,svg"),
        );
      const typography = modelLabel && doc.defaultView?.getComputedStyle?.(modelLabel);
      if (typography) {
        const css = `color:${typography.color};font-family:${typography.fontFamily};font-size:${typography.fontSize};font-weight:${typography.fontWeight};font-style:${typography.fontStyle};line-height:${typography.lineHeight};letter-spacing:${typography.letterSpacing};`;
        if (control.trigger.getAttribute("data-sp-typography") !== css) {
          control.trigger.style.cssText = css;
          control.trigger.setAttribute("data-sp-typography", css);
        }
      }
      const skills = state.get(stateKey(root));
      const text = pillLabel(skills);
      if (control.label.textContent !== text) {
        // Dim separators as in the mockup; textContent stays equal to pillLabel.
        const parts = text.split(" · ");
        control.label.textContent = "";
        parts.forEach((part, index) => {
          if (index) {
            const sep = doc.createElement("span");
            sep.setAttribute("data-sp-sep", "");
            sep.textContent = " · ";
            control.label.append(sep);
          }
          control.label.append(part);
        });
      }
      const title = pillTitle(skills);
      control.trigger.setAttribute("title", title);
      control.trigger.setAttribute("aria-label", title);
      wrapper.setAttribute("data-sp-state", skills?.length ? "active" : skills ? "empty" : "off");
      paintMenu(control.menu, skills ?? []);
      if (skills) control.trigger.removeAttribute("disabled");
      else {
        control.trigger.setAttribute("disabled", "");
        close(wrapper);
      }
    }
  }

  const unsubscribe = state.subscribe(update);
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
      observer?.disconnect();
      clearTimeout(timer);
      doc.body.removeEventListener("pointerdown", outside, true);
      for (const wrapper of controls.keys()) wrapper.remove();
      controls.clear();
      style.remove();
    },
  };
}
