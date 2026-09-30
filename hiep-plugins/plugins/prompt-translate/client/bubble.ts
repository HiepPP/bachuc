import { hasVietnamese } from "../shared/vietnamese";
import { reactProps, type Doc, type El, type Observer } from "./dom";

export type BubbleApi = {
  translate(text: string, cacheOnly: boolean): Promise<string | null>;
  original(text: string): Promise<string | null>;
};
export type BubbleOptions = { enabled(): boolean; activeSince(): number };

const TEXT = '[data-testid="user-message"] [data-message-text="true"]';
const MARK = "data-prompt-translate";
// Colors come from currentColor so the annotation follows every host theme.
const STYLE = `
[${MARK}] {display:flex;flex-direction:column;gap:2px;margin-top:6px;padding-top:6px;border-top:1px solid color-mix(in srgb,currentColor 18%,transparent);font-size:.92em;line-height:1.45;}
[${MARK}] .pt-label {font-size:10px;font-weight:600;letter-spacing:.06em;opacity:.55;}
[${MARK}] .pt-text {opacity:.78;white-space:pre-wrap;user-select:text;}
[${MARK}] .pt-loading {height:.9em;width:60%;border-radius:4px;background:currentColor;opacity:.12;animation:pt-pulse 1.2s ease-in-out infinite;}
[${MARK}] button {all:unset;cursor:pointer;opacity:.75;text-decoration:underline;}
@keyframes pt-pulse {50% {opacity:.05;}}
`;

export function installBubbles(
  api: BubbleApi,
  options: BubbleOptions,
  doc: Doc,
  Observer?: Observer,
) {
  let seen = new WeakMap<El, string>();
  let stopped = false;
  let scheduled = false;
  const style = doc.createElement("style");
  style.setAttribute(`${MARK}-style`, "");
  style.textContent = STYLE;
  doc.head.append(style);

  const element = (tag: string, className: string, text?: string) => {
    const node = doc.createElement(tag);
    node.setAttribute("class", className);
    if (text !== undefined) node.textContent = text;
    return node;
  };
  const existing = (textNode: El) => {
    const next = textNode.nextElementSibling;
    return next?.hasAttribute(MARK) ? next : null;
  };
  const box = (textNode: El, ...children: El[]) => {
    let node = existing(textNode);
    if (!node) {
      node = doc.createElement("div");
      node.setAttribute(MARK, "");
      const color = doc.defaultView?.getComputedStyle?.(textNode).color;
      if (color) node.style.cssText = `color:${color}`;
      textNode.after(node);
    }
    node.textContent = "";
    node.append(...children);
  };
  const show = (textNode: El, label: string, text: string) =>
    box(textNode, element("span", "pt-label", label), element("span", "pt-text", text));
  const drop = (textNode: El) => existing(textNode)?.remove();
  const current = (textNode: El, text: string) =>
    !stopped && textNode.isConnected && seen.get(textNode) === text;

  async function process(textNode: El, text: string): Promise<void> {
    let cacheOnly = true;
    try {
      const original = await api.original(text);
      if (!current(textNode, text)) return;
      if (original) return show(textNode, "VI gốc", original);
      if (!hasVietnamese(text)) return drop(textNode);
      const props = reactProps(
        textNode,
        (candidate) =>
          typeof candidate.message === "string" && typeof candidate.timestamp === "number",
      );
      // Prompts older than activation, or without a readable timestamp, never spend quota.
      cacheOnly = !props || (props.timestamp as number) < options.activeSince();
      if (!cacheOnly)
        box(textNode, element("span", "pt-label", "EN"), element("div", "pt-loading"));
      const translation = await api.translate(text, cacheOnly);
      if (!current(textNode, text)) return;
      if (translation) show(textNode, "EN", translation);
      else drop(textNode);
    } catch {
      if (!current(textNode, text)) return;
      if (cacheOnly) return drop(textNode);
      const retry = element("button", "pt-retry", "Thử lại");
      retry.addEventListener("click", () => void process(textNode, text));
      box(textNode, element("span", "pt-text", "Không dịch được · "), retry);
    }
  }

  function clearAll() {
    for (const node of Array.from(doc.querySelectorAll(`[${MARK}]`))) node.remove();
    seen = new WeakMap();
  }

  function scan() {
    scheduled = false;
    if (stopped) return;
    if (!options.enabled()) return clearAll();
    for (const textNode of Array.from(doc.querySelectorAll(TEXT))) {
      const text = (textNode.textContent ?? "").trim();
      if (!text || seen.get(textNode) === text) continue;
      seen.set(textNode, text);
      drop(textNode);
      void process(textNode, text);
    }
  }

  const observer = Observer
    ? new Observer(() => {
        if (scheduled || stopped) return;
        scheduled = true;
        setTimeout(scan, 50);
      })
    : undefined;
  observer?.observe(doc.body, { childList: true, subtree: true, characterData: true });
  scan();
  return {
    scan,
    stop() {
      stopped = true;
      observer?.disconnect();
      clearAll();
      style.remove();
    },
  };
}
