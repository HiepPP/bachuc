import { createWheelGesture } from "./gesture";
type Wheel = {
  deltaX: number;
  deltaY: number;
  deltaMode: number;
  ctrlKey: boolean;
  preventDefault(): void;
};
type Element = {
  addEventListener(name: string, fn: (e: Wheel) => void, opts: { passive: boolean }): void;
  removeEventListener(name: string, fn: (e: Wheel) => void): void;
};
export function bindWheel(element: unknown, switchSpace: (direction: number) => void) {
  if (typeof document === "undefined" || !element || !("addEventListener" in Object(element)))
    return () => {};
  const node = element as Element;
  const gesture = createWheelGesture();
  const listener = (e: Wheel) => {
    if (e.ctrlKey) return;
    if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) e.preventDefault();
    const scale = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 800 : 1;
    const direction = gesture(e.deltaX * scale, e.deltaY * scale, Date.now());
    if (direction) switchSpace(direction);
  };
  node.addEventListener("wheel", listener, { passive: false });
  return () => node.removeEventListener("wheel", listener);
}

import type { SidebarController } from "./sidebar-state";
import { matchProject, sidebarMembership, viewMembership } from "./sidebar-state";
import { adjacent, removalTarget } from "../shared/spaces";

// Desktop-only DOM adapter. No DOM globals leak into the native client bundle.
interface SlideAnimation {
  finished: Promise<unknown>;
  cancel(): void;
}
export interface DomNode {
  animate?(
    frames: { transform: string; opacity: number }[],
    options: { duration: number; easing: string; fill: string },
  ): SlideAnimation;
  dispatchEvent(event: DomEvent): boolean;
  firstChild: { nodeValue: string | null } | null;
  value?: string;
  focus?(): void;
  select?(): void;
  textContent: string | null;
  parentElement: DomNode | null;
  isConnected: boolean;
  style: { cssText: string; setProperty(name: string, value: string): void };
  appendChild(node: DomNode): void;
  remove(): void;
  contains(node: DomNode): boolean;
  querySelector(selector: string): DomNode | null;
  querySelectorAll(selector: string): ArrayLike<DomNode>;
  closest(selector: string): DomNode | null;
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
  addEventListener(name: string, listener: (e: DomEvent) => void, options?: unknown): void;
  removeEventListener(name: string, listener: (e: DomEvent) => void, options?: unknown): void;
}
interface DomEvent {
  target: DomNode;
  key?: string;
  deltaX?: number;
  deltaY?: number;
  deltaMode?: number;
  ctrlKey?: boolean;
  buttons?: number;
  preventDefault(): void;
}
export interface DomDocument extends DomNode {
  defaultView?: { matchMedia?(query: string): { matches: boolean } };
  createEvent(
    type: string,
  ): DomEvent & { initEvent(type: string, bubbles: boolean, cancelable: boolean): void };
  createElement(tag: string): DomNode;
  head: DomNode;
  body: DomNode;
}
interface Observer {
  observe(node: DomNode, options: unknown): void;
  disconnect(): void;
}
declare const document: DomDocument;
declare const navigator: { userAgent: string };
declare const MutationObserver: new (callback: () => void) => Observer;
const OWNER = "data-paseo-spaces-owner";
const HIDDEN = "data-paseo-space-hidden";

export function supportsSidebarInjection() {
  return (
    typeof document !== "undefined" &&
    typeof navigator !== "undefined" &&
    /Electron\//.test(navigator.userAgent)
  );
}
export function installSidebar(controller: SidebarController) {
  if (!supportsSidebarInjection()) return () => {};
  return mountSidebar(document, (callback) => new MutationObserver(callback), controller);
}
export function mountSidebar(
  doc: DomDocument,
  observe: (fn: () => void) => Observer,
  controller: SidebarController,
) {
  // Multiple host installations must not compete over the same app-owned sidebar.
  if (doc.querySelector(`[${OWNER}]`)) return () => {};
  const css = doc.createElement("style");
  css.setAttribute(OWNER, "true");
  css.textContent = `[${HIDDEN}]{display:none!important}.paseo-spaces-bar{flex-shrink:0;padding:6px 10px;display:flex;flex-direction:column;gap:0;border-top:1px solid #8883;font:13px system-ui;color:inherit}.paseo-spaces-tabs{display:flex;gap:4px;overflow-x:auto;padding:2px}.paseo-spaces-bar [role=status]:empty{display:none}.paseo-spaces-bar .paseo-spaces-tabs button{min-width:26px;height:26px;padding:0 7px;border-radius:6px;font-size:12px;line-height:24px}.paseo-spaces-bar button,.paseo-spaces-menu button{font:inherit;color:inherit;background:transparent;border:1px solid #8883;border-radius:8px;padding:8px 12px;cursor:pointer;flex-shrink:0}.paseo-spaces-bar button[aria-selected=true]{background:#7660de;color:white;border-color:transparent}.paseo-spaces-bar button:focus-visible,.paseo-spaces-menu button:focus-visible{outline:2px solid #7660de;outline-offset:2px}.paseo-spaces-menu{display:flex;flex-direction:column;gap:2px;border-top:1px solid #8883;margin-top:4px;padding:6px 4px 2px;font:12px/1.3 system-ui;color:inherit}.paseo-spaces-menu>div{padding:2px 8px 5px;font-size:10px;font-weight:500;opacity:.6}.paseo-spaces-menu button{display:flex;align-items:center;justify-content:space-between;width:100%;min-height:28px;padding:5px 8px;border:0;border-radius:5px;text-align:left;font:inherit;line-height:18px}.paseo-spaces-menu button:hover:not(:disabled){background:#8882}.paseo-spaces-menu button[aria-checked=true]{background:#7660de14;font-weight:500}.paseo-spaces-menu button[aria-checked=true]::after{content:"✓";font-size:12px;color:#8b79df}.paseo-spaces-bar button:disabled,.paseo-spaces-menu button:disabled{opacity:.5;cursor:default}`;
  css.textContent +=
    "[data-paseo-space-heading]{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;min-width:0}";
  doc.head.appendChild(css);
  const bar = doc.createElement("div");
  bar.setAttribute("class", "paseo-spaces-bar");
  bar.setAttribute("data-testid", "spaces-sidebar-controls");
  const tabs = doc.createElement("div");
  tabs.setAttribute("class", "paseo-spaces-tabs");
  tabs.setAttribute("role", "tablist");
  tabs.setAttribute("aria-label", "Project Spaces");
  const error = doc.createElement("div");
  error.setAttribute("role", "status");
  bar.appendChild(tabs);
  bar.appendChild(error);
  const actions = doc.createElement("div");
  actions.setAttribute("class", "paseo-spaces-menu");
  actions.setAttribute("role", "menu");
  const closeActions = () => {
    actions.setAttribute("role", "menu");
    actions.remove();
    actions.textContent = "";
  };
  const headings = new Map<DomNode, { text: string; title: string | null }>();
  function updateHeading(name: string | null) {
    const marker = scroll?.querySelector('[data-testid="sidebar-display-preferences-menu"]');
    let parent = marker?.parentElement;
    while (parent && parent !== scroll) {
      const heading = Array.from(parent.querySelectorAll("div,span")).find(
        (node) =>
          !node.querySelector("*") && (headings.has(node) || node.textContent === "Workspaces"),
      );
      if (heading) {
        if (!headings.has(heading))
          headings.set(heading, {
            text: heading.textContent ?? "Workspaces",
            title: heading.getAttribute("title"),
          });
        const original = headings.get(heading)!;
        const text = name ?? original.text;
        // Preserve React's text node so subsequent host renders can still update it.
        if (heading.firstChild && heading.textContent !== text) heading.firstChild.nodeValue = text;
        heading.setAttribute("data-paseo-space-heading", "true");
        if (heading.getAttribute("title") !== text) heading.setAttribute("title", text);
        break;
      }
      parent = parent.parentElement;
    }
    for (const heading of headings.keys()) if (!heading.isConnected) headings.delete(heading);
  }
  const hidden = new Set<DomNode>();
  const menus = new Set<DomNode>();
  let scroll: DomNode | null = null,
    active = "space-1",
    stopped = false,
    scheduled = false,
    signature = "",
    dragging = false;
  let previousOrder: string[] = [];
  let unbindWheel = () => {};
  function hide(node: DomNode, value: boolean) {
    if (value && !node.getAttribute(HIDDEN)) {
      node.setAttribute(HIDDEN, "true");
      hidden.add(node);
    }
    if (!value && hidden.has(node)) {
      node.removeAttribute(HIDDEN);
      hidden.delete(node);
    }
  }
  function button(parent: DomNode, title: string, action: () => void, disabled = false) {
    const b = doc.createElement("button");
    b.setAttribute("type", "button");
    b.textContent = title;
    if (disabled) b.setAttribute("disabled", "");
    b.addEventListener("click", action);
    parent.appendChild(b);
    return b;
  }
  let slide: SlideAnimation | undefined;
  let slideVersion = 0;
  let pendingSpace: string | null = null;
  function cancelSlide() {
    slideVersion++;
    slide?.cancel();
    slide = undefined;
    pendingSpace = null;
  }
  function select(id: string) {
    closeActions();
    const spaces = controller.get()?.state.spaces ?? [];
    cancelSlide();
    if (id === active || !spaces.some((space) => space.id === id)) return;
    const direction =
      spaces.findIndex((space) => space.id === id) >
      spaces.findIndex((space) => space.id === active)
        ? 1
        : -1;
    const node = scroll;
    const apply = () => {
      active = id;
      signature = "";
      render();
    };
    if (
      !node?.animate ||
      doc.defaultView?.matchMedia?.("(prefers-reduced-motion: reduce)").matches
    ) {
      apply();
      return;
    }
    pendingSpace = id;
    const version = slideVersion;
    slide = node.animate(
      [
        { transform: "translateX(0)", opacity: 1 },
        { transform: `translateX(${-direction * 100}%)`, opacity: 0 },
      ],
      { duration: 65, easing: "cubic-bezier(.4,0,1,1)", fill: "forwards" },
    );
    void slide.finished
      .then(() => {
        if (stopped || version !== slideVersion) return;
        slide?.cancel();
        pendingSpace = null;
        apply();
        if (!node.isConnected) return;
        slide = node.animate!(
          [
            { transform: `translateX(${direction * 100}%)`, opacity: 0 },
            { transform: "translateX(0)", opacity: 1 },
          ],
          { duration: 105, easing: "cubic-bezier(.16,1,.3,1)", fill: "none" },
        );
        void slide.finished.catch(() => {});
      })
      .catch(() => {});
  }
  function render() {
    if (stopped) return;
    const snap = controller.get();
    const nextScroll = doc.querySelector('[data-testid="sidebar-project-workspace-list-scroll"]');
    if (nextScroll !== scroll) {
      unbindWheel();
      scroll = nextScroll;
      if (scroll?.parentElement) {
        scroll.parentElement.appendChild(bar);
        const gesture = createWheelGesture();
        const listener = (e: DomEvent) => {
          if (
            bar.contains(e.target) ||
            dragging ||
            e.buttons ||
            e.ctrlKey ||
            controller.get()?.busy
          )
            return;
          const dx = e.deltaX ?? 0,
            dy = e.deltaY ?? 0;
          if (Math.abs(dx) > Math.abs(dy)) e.preventDefault();
          const scale = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 800 : 1;
          const direction = gesture(dx * scale, dy * scale, Date.now());
          const current = controller.get();
          if (direction && current)
            select(
              adjacent(
                current.state.spaces.map((s) => s.id),
                pendingSpace ?? active,
                direction,
              ),
            );
        };
        // Stay reachable while the list is translated or the Space is empty.
        const target = scroll.parentElement;
        target.addEventListener("wheel", listener, { passive: false });
        unbindWheel = () => target.removeEventListener("wheel", listener);
      }
    }
    if (!snap) {
      const message = controller.getLoadError();
      if (message && signature !== message) {
        signature = message;
        tabs.textContent = "";
        error.textContent = `${message} Use Refresh to retry.`;
        button(tabs, "Refresh", () => {
          void controller.refresh();
        });
      }
      return;
    }
    if (!snap.state.spaces.some((s) => s.id === active)) {
      const previous = previousOrder.slice(0, previousOrder.indexOf(active)).reverse();
      active =
        previous.find((id) => snap.state.spaces.some((s) => s.id === id)) ??
        snap.state.spaces[0].id;
    }
    updateHeading(snap.error ? null : snap.state.spaces.find((space) => space.id === active)!.name);
    previousOrder = snap.state.spaces.map((s) => s.id);
    const stamp = JSON.stringify([active, snap.state.spaces, snap.busy, snap.error]);
    if (stamp !== signature) {
      signature = stamp;
      tabs.textContent = "";
      error.textContent = snap.error ? `${snap.error} Use Refresh to retry.` : "";
      for (const [index, space] of snap.state.spaces.entries()) {
        const b = button(tabs, String(index + 1), () => select(space.id));
        b.setAttribute("role", "tab");
        b.setAttribute("aria-label", space.name);
        b.setAttribute("title", space.name);
        b.setAttribute("aria-selected", String(space.id === active));
        const openActions = () => {
          closeActions();
          const current = controller.get();
          if (!current) return;
          const rename = button(
            actions,
            "Rename workspace",
            () => {
              closeActions();
              actions.setAttribute("role", "group");
              const input = doc.createElement("input");
              input.setAttribute("aria-label", "Workspace name");
              input.value = space.name;
              input.style.cssText =
                "font:inherit;color:inherit;background:transparent;border:1px solid #8886;border-radius:5px;padding:6px;min-width:0;width:100%;box-sizing:border-box";
              actions.appendChild(input);
              let saving = false;
              const save = async () => {
                if (saving || !input.value?.trim()) return;
                saving = true;
                input.setAttribute("disabled", "");
                submit.setAttribute("disabled", "");
                const ok = await controller.rename(space.id, input.value);
                saving = false;
                if (stopped || !input.isConnected) return;
                input.removeAttribute("disabled");
                submit.removeAttribute("disabled");
                if (ok) closeActions();
                else input.focus?.();
              };
              const submit = button(actions, "Save", () => {
                void save();
              });
              button(actions, "Cancel", closeActions);
              input.addEventListener("input", () => {
                if (input.value?.trim()) submit.removeAttribute("disabled");
                else submit.setAttribute("disabled", "");
              });
              input.addEventListener("keydown", (e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void save();
                }
              });
              bar.appendChild(actions);
              input.focus?.();
              input.select?.();
            },
            current.busy,
          );
          rename.setAttribute("role", "menuitem");
          const last = current.state.spaces.length === 1;
          const target = last ? null : removalTarget(current.state, space.id);
          const name = current.state.spaces.find((s) => s.id === target)?.name;
          const remove = button(
            actions,
            last ? "Keep at least one workspace" : `Remove ${space.name} → ${name}`,
            () => {
              closeActions();
              void controller.remove(space.id).then((ok) => {
                if (ok && active === space.id && target) select(target);
              });
            },
            last || current.busy,
          );
          remove.setAttribute("role", "menuitem");
          remove.setAttribute("aria-label", `Remove ${space.name}`);
          bar.appendChild(actions);
        };
        b.addEventListener("contextmenu", (e) => {
          e.preventDefault();
          openActions();
        });
        b.addEventListener("keydown", (e) => {
          if (e.key === "Delete" || e.key === "ContextMenu") {
            e.preventDefault();
            openActions();
          }
        });
      }
      const add = button(
        tabs,
        "+",
        () => {
          void controller.create().then((ok) => {
            const s = controller.get();
            if (ok && s) select(s.state.spaces[s.state.spaces.length - 1].id);
          });
        },
        snap.busy,
      );
      add.setAttribute("aria-label", "Create Space");
      add.setAttribute("title", "Create Space");
      if (snap.error)
        button(tabs, "Refresh", () => {
          void controller.refresh();
        });
    }
    // Stable sidebar view keys also cover projects from other connected hosts.
    if (scroll)
      for (const row of Array.from(
        scroll.querySelectorAll('[data-testid^="sidebar-project-row-"]'),
      )) {
        const key = (row.getAttribute("data-testid") ?? "").slice("sidebar-project-row-".length);
        const project = matchProject(key, snap.projects);
        const group = row.closest('[role="group"]');
        if (group && scroll.contains(group))
          hide(group, viewMembership(snap.state, key, project) !== active && !snap.error);
      }
    // Pinned workspace rows are outside project groups and need their own mapping.
    const pinned = doc.querySelector('[data-testid="sidebar-pinned-section"]');
    if (pinned)
      for (const row of Array.from(
        pinned.querySelectorAll('[data-testid^="sidebar-workspace-row-"]'),
      )) {
        const key = (row.getAttribute("data-testid") ?? "").slice("sidebar-workspace-row-".length);
        let id = key;
        try {
          id = JSON.parse(key)[1] ?? key;
        } catch {
          /* Keep plain workspace IDs. */
        }
        const project = snap.projects.find((p) => p.workspaces.some((w) => w.id === id));
        if (project) hide(row, sidebarMembership(snap.state, project.id) !== active && !snap.error);
      }
    for (const marker of Array.from(
      doc.querySelectorAll('[data-testid^="sidebar-project-menu-open-settings-"]'),
    )) {
      const key = (marker.getAttribute("data-testid") ?? "").slice(
        "sidebar-project-menu-open-settings-".length,
      );
      const project = matchProject(key, snap.projects);
      const menu =
        marker.closest('[role="menu"]') ??
        (marker.closest('[data-menu-surface="true"]') ? marker.parentElement : null);
      if (!menu) continue;
      let own = menu.querySelector(".paseo-spaces-menu");
      const menuStamp = JSON.stringify([
        key,
        snap.state.spaces,
        viewMembership(snap.state, key, project),
        snap.busy,
      ]);
      if (own?.getAttribute("data-state") === menuStamp) continue;
      if (!own) {
        own = doc.createElement("div");
        own.setAttribute("class", "paseo-spaces-menu");
        menu.appendChild(own);
        menus.add(own);
      }
      own.setAttribute("data-state", menuStamp);
      own.textContent = "";
      const heading = doc.createElement("div");
      heading.textContent = "Move to workspace";
      own.appendChild(heading);
      for (const space of snap.state.spaces) {
        const selected = viewMembership(snap.state, key, project) === space.id;
        const b = button(
          own,
          space.name,
          () => {
            void controller.moveView(key, space.id).then((ok) => {
              if (!ok || stopped || !menu.isConnected) return;
              // Let the host close its overlay and restore focus through its Escape handler.
              const event = doc.createEvent("Event");
              event.initEvent("keydown", true, true);
              event.key = "Escape";
              menu.dispatchEvent(event);
            });
          },
          snap.busy,
        );
        b.setAttribute("role", "menuitemradio");
        b.setAttribute("aria-checked", String(selected));
        b.setAttribute("data-menu-item", "true");
        b.setAttribute("data-menu-disabled", String(snap.busy));
      }
    }
    for (const node of hidden)
      if (!node.isConnected) {
        node.removeAttribute(HIDDEN);
        hidden.delete(node);
      }
    for (const node of menus) if (!node.isConnected) menus.delete(node);
  }
  const observer = observe(() => {
    if (scheduled || stopped) return;
    scheduled = true;
    queueMicrotask(() => {
      scheduled = false;
      render();
    });
  });
  observer.observe(doc.body, { childList: true, subtree: true });
  const onDrag = () => {
      dragging = true;
    },
    onEnd = () => {
      dragging = false;
    };
  const dismiss = (e: DomEvent) => {
    if (!actions.contains(e.target)) closeActions();
  };
  const escape = (e: DomEvent) => {
    if (e.key === "Escape") closeActions();
  };
  doc.addEventListener("pointerdown", dismiss);
  doc.addEventListener("keydown", escape);
  doc.addEventListener("dragstart", onDrag);
  doc.addEventListener("dragend", onEnd);
  doc.addEventListener("pointerup", onEnd);
  const unsubscribe = controller.subscribe(render);
  void controller.refresh();
  const timer = setInterval(() => {
    void controller.refresh();
  }, 15000);
  return () => {
    stopped = true;
    cancelSlide();
    clearInterval(timer);
    observer.disconnect();
    unsubscribe();
    unbindWheel();
    doc.removeEventListener("dragstart", onDrag);
    doc.removeEventListener("dragend", onEnd);
    doc.removeEventListener("pointerup", onEnd);
    for (const node of hidden) node.removeAttribute(HIDDEN);
    for (const node of menus) node.remove();
    doc.removeEventListener("pointerdown", dismiss);
    doc.removeEventListener("keydown", escape);
    closeActions();
    bar.remove();
    for (const [heading, original] of headings) {
      if (heading.firstChild) heading.firstChild.nodeValue = original.text;
      heading.removeAttribute("data-paseo-space-heading");
      if (original.title === null) heading.removeAttribute("title");
      else heading.setAttribute("title", original.title);
    }
    css.remove();
    controller.stop();
  };
}
