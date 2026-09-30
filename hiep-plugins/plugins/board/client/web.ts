import { Platform } from "react-native";

interface Element {
  parentElement: Element | null;
  offsetHeight: number;
  lastElementChild: Element | null;
  isConnected: boolean;
  textContent: string | null;
  style: { cssText: string };
  closest(selector: string): Element | null;
  querySelectorAll(selector: string): Iterable<Element>;
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
  appendChild(node: Element): void;
  remove(): void;
  click(): void;
  cloneNode(deep: boolean): Element;
  addEventListener(name: string, listener: (event: Event) => void): void;
}
interface Event {
  key?: string;
  preventDefault(): void;
  stopPropagation(): void;
}
declare const document: {
  head: Element;
  querySelector(selector: string): Element | null;
  querySelectorAll(selector: string): Iterable<Element>;
  createElement(tag: string): Element;
};
declare const history: { pushState(state: unknown, title: string, url: string): void };
declare const PopStateEvent: new (type: string) => unknown;
declare function dispatchEvent(event: unknown): void;

// Each connected host registers shortcuts. Let Paseo's sidebar choose the active/remembered
// host instead of opening whichever installation happened to register its listener first.
export function openBoardFromSidebar(): boolean {
  if (Platform.OS !== "web" || typeof document === "undefined") return false;
  const button = document.querySelector('[data-testid="plugin-sidebar-board-board"]');
  if (!button) return false;
  button.click();
  return true;
}

/**
 * Mirrors the sidebar project "+" button: click it when rendered, otherwise push the same
 * `/new?...` route the app builds for it. Returns false when neither is possible.
 */
export function openNewWorkspaceForProject(input: {
  serverId: string;
  cwd: string;
  name: string;
  projectId?: string;
}): boolean {
  if (Platform.OS !== "web" || typeof document === "undefined") return false;
  const label = `Create a new workspace for ${input.name}`.replace(/"/g, '\\"');
  const button = document.querySelector(`[aria-label="${label}"]`);
  if (button && button.getAttribute("aria-disabled") !== "true") {
    button.click();
    return true;
  }
  const query = new URLSearchParams({ serverId: input.serverId, dir: input.cwd, name: input.name });
  if (input.projectId) query.set("projectId", input.projectId);
  history.pushState(null, "", `/new?${query.toString()}`);
  dispatchEvent(new PopStateEvent("popstate"));
  return true;
}

// Top-right stack order for the desktop thread actions.
const STACK = ["Remove from Board", "Remove and Start New Thread", "Jump To Parent"];
const STACK_SELECTOR = `:is(${STACK.map((label) => `[aria-label="${label}"]`).join(",")})`;

// Keep Paseo's agent-scoped action and pending/error handling; only its desktop placement changes.
export function installRemovePlacement() {
  if (Platform.OS !== "web" || typeof document === "undefined") return () => {};
  const css = document.createElement("style");
  css.textContent =
    `[data-testid^="workspace-pane-"] [role="button"]${STACK_SELECTOR}:not([data-board-remove-copy]){display:none!important}` +
    "[data-board-parent-copy], [data-board-parent-copy] *{color:#f97316!important}" +
    "[data-board-parent-copy]{border-color:#f97316!important}";
  document.head.appendChild(css);
  const copies = new Map<Element, Element>();
  function sync() {
    for (const [source, copy] of copies) {
      if (!source.isConnected) {
        copy.remove();
        copies.delete(source);
      }
    }
    for (const source of document.querySelectorAll(`[role="button"]${STACK_SELECTOR}`)) {
      if (source.getAttribute("data-board-remove-copy")) continue;
      const pane = source.closest('[data-testid^="workspace-pane-"]');
      const content = pane?.lastElementChild;
      if (!content) continue;
      // Attach inside the retained tab so Paseo hides it together with that thread.
      let tab = source;
      while (tab.parentElement && tab.parentElement !== content) tab = tab.parentElement;
      if (tab.parentElement !== content) continue;
      let copy = copies.get(source);
      if (!copy) {
        copy = source.cloneNode(true);
        copy.setAttribute("data-board-remove-copy", "true");
        if (source.getAttribute("aria-label") === "Jump To Parent")
          copy.setAttribute("data-board-parent-copy", "true");
        copy.style.cssText += ";position:absolute;top:12px;right:12px;z-index:10;margin:0;";
        const activate = (event: Event) => {
          event.preventDefault();
          event.stopPropagation();
          if (source.isConnected && source.getAttribute("aria-disabled") !== "true") source.click();
        };
        copy.addEventListener("click", activate);
        copy.addEventListener("keydown", (event) => {
          if (event.key === "Enter" || event.key === " ") activate(event);
        });
        tab.appendChild(copy);
        copies.set(source, copy);
      }
      let top = 12;
      for (const label of STACK.slice(0, STACK.indexOf(source.getAttribute("aria-label")!))) {
        const above = [
          ...tab.querySelectorAll(`[data-board-remove-copy][aria-label="${label}"]`),
        ][0];
        if (above) top += above.offsetHeight + 8;
      }
      copy.style.cssText += `;top:${top}px;`;
      const disabled = source.getAttribute("aria-disabled") === "true";
      if (copy.getAttribute("aria-disabled") !== String(disabled)) {
        copy.setAttribute("aria-disabled", String(disabled));
        copy.setAttribute("tabindex", disabled ? "-1" : "0");
        copy.style.cssText += `;opacity:${disabled ? "0.5" : "1"};pointer-events:${disabled ? "none" : "auto"};`;
      }
    }
  }
  sync();
  const timer = setInterval(sync, 250);
  return () => {
    clearInterval(timer);
    for (const copy of copies.values()) copy.remove();
    copies.clear();
    css.remove();
  };
}
