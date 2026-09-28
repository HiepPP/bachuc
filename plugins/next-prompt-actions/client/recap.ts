import type { Document, Node } from "./web";

const TAG = "data-paseo-markdown-tag";
const blocks = new Set(["ul", "ol", "p", "pre", "blockquote", "table", "hr"]);
const heading = /^h[1-6]$/;
const next = /^what(?:['’]s)? next$|^next steps$/i;

export const recapStyles = `
[data-npa-recap-heading] {font-size:22px!important;line-height:1.3!important;margin-top:20px!important;margin-bottom:12px!important;padding-bottom:0!important;border-bottom-width:0!important;}
[data-npa-recap-heading] * {font-size:inherit!important;line-height:inherit!important;}
[data-npa-recap] {display:grid!important;grid-template-columns:auto minmax(0,max-content) auto;justify-content:start;align-items:baseline;gap:0!important;margin:0!important;padding:0 0 20px!important;border-bottom:1px solid color-mix(in srgb,currentColor 14%,transparent);}
[data-npa-recap-field] {display:block!important;min-width:0!important;margin:0!important;padding:0 24px!important;border-left:1px solid color-mix(in srgb,currentColor 16%,transparent);font:14px/1.6 system-ui!important;overflow-wrap:anywhere;text-wrap:pretty;}
[data-npa-recap-field="did"] {max-width:68ch;}
[data-npa-recap-field]:first-child {padding-left:0!important;border-left:0;}
[data-npa-recap-field]:last-child {padding-right:0!important;}
[data-npa-recap-field] * {font-size:inherit!important;line-height:inherit!important;}
[data-npa-recap-field] > [data-paseo-markdown-list-marker] {display:none!important;}
[data-npa-recap-field] [data-paseo-markdown-tag="p"] {margin:0!important;}
[data-npa-recap-field] [data-paseo-markdown-tag="code"] {border-radius:6px;padding:1px 5px!important;background:color-mix(in srgb,currentColor 6%,transparent)!important;}
[data-npa-folded] {display:none!important;}
@media(max-width:800px) {
  [data-npa-recap] {grid-template-columns:minmax(0,1fr);gap:10px!important;}
  [data-npa-recap-field] {padding:0!important;border-left:0;}
}
`;

/** Decorate known Markdown shapes without moving React-owned nodes or changing copy text. */
export function decorateRecap(message: Node): (() => void) | null {
  const nodes = Array.from(message.querySelectorAll(`[${TAG}]`));
  const start = nodes.findIndex(
    (node) =>
      /^h[1-6]$/.test(node.getAttribute(TAG) ?? "") &&
      /^recap$/i.test(node.textContent?.trim() ?? "") &&
      !node.closest(`[${TAG}="blockquote"]`),
  );
  if (start < 0) return null;
  let end = start + 1;
  while (end < nodes.length && !/^h[1-6]$/.test(nodes[end].getAttribute(TAG) ?? "")) end++;
  const section = nodes.slice(start + 1, end);
  const topBlocks = section.filter((node) => {
    if (!blocks.has(node.getAttribute(TAG) ?? "")) return false;
    for (
      let parent = node.parentElement;
      parent && parent !== message;
      parent = parent.parentElement
    )
      if (blocks.has(parent.getAttribute(TAG) ?? "")) return false;
    return true;
  });
  if (topBlocks.length !== 1 || topBlocks[0].getAttribute(TAG) !== "ul") return null;
  const list = topBlocks[0];
  const fields = items(list);
  // Only Did may carry a nested list, e.g. one bullet per change.
  const nested = Array.from(list.querySelectorAll(`[${TAG}="ul"], [${TAG}="ol"]`));
  if (fields.length !== 3 || !nested.every((node) => fields[1].contains?.(node))) return null;
  const labels = ["branch", "did", "commit/push"];
  if (
    !fields.every((field, index) => {
      const content = (field.textContent ?? "").replace(/^\s*[•*-]?\s*/, "");
      const match = /^(Branch|Did|Commit\/push):\s*([\s\S]+)$/i.exec(content.trim());
      return match?.[1].toLowerCase() === labels[index] && !!match[2].trim();
    })
  )
    return null;
  const changed: { node: Node; name: string; before: string | null }[] = [];
  const mark = (node: Node, name: string, value: string) => {
    changed.push({ node, name, before: node.getAttribute(name) });
    node.setAttribute(name, value);
  };
  mark(nodes[start], "data-npa-recap-heading", "true");
  mark(list, "data-npa-recap", "true");
  fields.forEach((field, index) => mark(field, "data-npa-recap-field", labels[index]));
  return () => {
    for (const { node, name, before } of changed) {
      if (before === null) node.removeAttribute(name);
      else node.setAttribute(name, before);
    }
  };
}

function items(list: Node) {
  return Array.from(list.querySelectorAll(`[${TAG}="li"]`)).filter((item) => {
    let parent = item.parentElement;
    while (parent && !/^[uo]l$/.test(parent.getAttribute(TAG) ?? "")) parent = parent.parentElement;
    return parent === list;
  });
}

// Paseo renders each Markdown block of one reply as its own history row; the rows share a message ID.
export function messageParts(message: Node): Node[] {
  const row = message.closest("[data-message-id]");
  const id = row?.getAttribute("data-message-id");
  const list = row?.parentElement;
  if (!id || !list) return [message];
  return Array.from(list.querySelectorAll('[data-testid="assistant-message"]')).filter(
    (part) => part.closest("[data-message-id]")?.getAttribute("data-message-id") === id,
  );
}

function topLevel(message: Node) {
  return Array.from(message.querySelectorAll(`[${TAG}]`)).filter((node) => {
    const tag = node.getAttribute(TAG) ?? "";
    if (!blocks.has(tag) && !heading.test(tag)) return false;
    for (
      let parent = node.parentElement;
      parent && parent !== message;
      parent = parent.parentElement
    ) {
      const outer = parent.getAttribute(TAG) ?? "";
      if (blocks.has(outer) || outer === "li" || heading.test(outer)) return false;
    }
    return true;
  });
}

// Clones keep the host's classes and inline styles but drop Markdown tags, so later scans and the
// host's copy logic never mistake them for message content.
function copy(node: Node) {
  const clone = node.cloneNode!(true);
  // Keep markers of nested list items; drop only the copied item's own marker.
  for (const marker of Array.from(clone.querySelectorAll("[data-paseo-markdown-list-marker]"))) {
    const item = marker.parentElement?.closest(`[${TAG}="li"]`);
    if (!item || item === clone) marker.remove();
  }
  for (const inner of [clone, ...Array.from(clone.querySelectorAll("*"))]) {
    const tag = inner.getAttribute(TAG) ?? "";
    if (tag === "code") inner.setAttribute("data-npa-code", "true");
    if (inner !== clone && (tag === "li" || /^[uo]l$/.test(tag)))
      inner.setAttribute("data-npa-list", tag);
    for (const name of [TAG, "data-npa-recap-field"]) inner.removeAttribute(name);
  }
  return clone;
}

function stripLabel(node: Node): boolean {
  for (const child of Array.from(node.childNodes ?? [])) {
    if (child.nodeType === 3) {
      if (!child.nodeValue?.trim()) continue;
      child.nodeValue = child.nodeValue.replace(/^\s*(?:Branch|Did|Commit\/push):\s*/i, "");
      return true;
    }
    if (stripLabel(child)) return true;
  }
  return false;
}

// Split cloned inline trees at Markdown line breaks; React-owned nodes stay untouched.
function lines(node: Node): Node[] {
  if (node.nodeType === 3) {
    return (node.nodeValue ?? "").split(/\r?\n/).map((text) => {
      const clone = node.cloneNode!(false);
      clone.nodeValue = text;
      return clone;
    });
  }
  const result = [node.cloneNode!(false)];
  for (const child of Array.from(node.childNodes ?? [])) {
    const parts = lines(child);
    parts.forEach((part, index) => {
      if (index) result.push(node.cloneNode!(false));
      result[result.length - 1].appendChild(part);
    });
  }
  return result;
}

function precedingRecap(tops: Node[], end: number) {
  let start = end;
  while (start > 0 && !heading.test(tops[start - 1].getAttribute(TAG) ?? "")) start--;
  const title = tops[start - 1];
  if (
    !title ||
    !heading.test(title.getAttribute(TAG) ?? "") ||
    !/^recap$/i.test(title.textContent?.trim() ?? "") ||
    title.closest(`[${TAG}="blockquote"]`)
  )
    return null;
  const paragraphs = tops.slice(start, end);
  const list =
    paragraphs.length === 1 && paragraphs[0].getAttribute(TAG) === "ul" ? paragraphs[0] : null;
  if (!list && !paragraphs.every((node) => node.getAttribute(TAG) === "p")) return null;
  const fields = list ? items(list) : paragraphs.flatMap(lines);
  if (
    list &&
    Array.from(list.querySelectorAll(`[${TAG}="ul"], [${TAG}="ol"]`)).some(
      (node) => !fields[1]?.contains?.(node),
    )
  )
    return null;
  const labels = ["Branch", "Did", "Commit/push"];
  if (
    fields.length !== 3 ||
    !fields.every((field, index) => {
      const match = /^(Branch|Did|Commit\/push):\s*(\S[\s\S]*)$/i.exec(
        (field.textContent ?? "").replace(/^\s*[•*-]?\s*/, "").trim(),
      );
      return match?.[1].toLowerCase() === labels[index].toLowerCase();
    })
  )
    return null;
  return { title, paragraphs, fields };
}

/** True when the nearest heading before `block` in the same reply is What Next or Next Steps. */
export function underNextHeading(message: Node, block: Node): boolean {
  const tops = messageParts(message).flatMap(topLevel);
  for (let index = tops.indexOf(block) - 1; index >= 0; index--)
    if (heading.test(tops[index].getAttribute(TAG) ?? ""))
      return next.test(tops[index].textContent?.trim() ?? "");
  return false;
}

/**
 * Show a directly preceding compact Recap, the What Next heading, and its intro paragraphs inside
 * the prompt panel. Native nodes are hidden in place, never moved, and restored by `undo`.
 * `intact` turns false when the host re-renders a hidden node, so the caller can fold again.
 */
export function foldPanel(
  doc: Document,
  message: Node,
  block: Node,
  panel: Node,
  section: Node,
  done = false,
): { undo(): void; intact(): boolean } {
  const parts = messageParts(message);
  const tops = parts.flatMap(topLevel);
  // The panel sits inside the prompt block and its text changes with every selection; only host
  // text marks a re-render.
  const text = (node: Node) => (node.contains?.(panel) ? ownText(node, panel) : node.textContent);
  const contents = new Map(tops.map((node) => [node, text(node)]));
  let index = tops.indexOf(block) - 1;
  const intro: Node[] = [];
  while (index >= 0 && tops[index].getAttribute(TAG) === "p") intro.unshift(tops[index--]);
  const title = tops[index];
  if (
    !title ||
    !heading.test(title.getAttribute(TAG) ?? "") ||
    !next.test(title.textContent?.trim() ?? "")
  )
    return { undo() {}, intact: () => true };
  const element = (tag: string, name: string, text?: string) => {
    const node = doc.createElement(tag);
    if (name) node.setAttribute("class", name);
    if (text !== undefined) node.textContent = text;
    return node;
  };
  const badge = () => {
    const node = element("span", "npa-badge");
    node.setAttribute("aria-hidden", "true");
    return node;
  };
  const value = (field: Node, into: Node) => {
    const clone = copy(field);
    stripLabel(clone);
    for (const child of Array.from(clone.childNodes ?? [])) into.appendChild(child);
    return into;
  };
  // The reply declared its goal done; everything suggested after it only closes or leaves the task.
  const goal = () => {
    const node = element("span", "npa-chip npa-goal", "Task done");
    node.setAttribute("aria-label", "Task done");
    return node;
  };
  const added: Node[] = [];
  const hidden = [title, ...intro];
  const parsed = precedingRecap(tops, index);
  // Host layout classes (flex, width, pre-wrap) squeeze chip text into one word per line.
  const chip = (field: Node, name: string) => {
    const into = value(field, element("span", name));
    for (const inner of Array.from(into.querySelectorAll("*"))) {
      inner.removeAttribute("class");
      inner.removeAttribute("style");
    }
    return into;
  };
  if (parsed) {
    const { title: recapTitle, paragraphs, fields } = parsed;
    hidden.unshift(recapTitle, ...paragraphs);
    const [branch, did, commit] = fields;
    const recap = element("div", "npa-section npa-recap");
    recap.setAttribute("role", "group");
    recap.setAttribute("aria-label", "Recap");
    const head = element("div", "npa-recap-head");
    const kicker = element("span", "npa-kicker");
    kicker.appendChild(badge());
    kicker.appendChild(element("span", "", recapTitle.textContent?.trim() || "Recap"));
    head.appendChild(kicker);
    const meta = element("span", "npa-meta");
    const branchChip = element("span", "npa-chip npa-branch");
    branchChip.appendChild(chip(branch, "npa-branch-value"));
    meta.appendChild(branchChip);
    const shown = chip(commit, "npa-commit-value");
    const said = shown.textContent?.trim() ?? "";
    const status = element(
      "span",
      /^(?:committed|pushed)\b/i.test(said) ? "npa-chip npa-commit npa-ok" : "npa-chip npa-commit",
    );
    status.setAttribute("aria-label", `Commit/push: ${said}`);
    // State reads from the icon and wording, not an extra hue (TASTE.md).
    status.appendChild(/^none\.?$/i.test(said) ? element("span", "", "No commit") : shown);
    meta.appendChild(status);
    if (done) meta.appendChild(goal());
    head.appendChild(meta);
    recap.appendChild(head);
    const bullets = did.querySelector(`[${TAG}="ul"], [${TAG}="ol"]`);
    recap.appendChild(value(did, element(bullets ? "div" : "p", "npa-did")));
    panel.prepend!(recap);
    added.push(recap);
  }
  const head = element("div", "npa-next-head");
  const name = element("div", "npa-next-title");
  name.appendChild(badge());
  name.appendChild(element("span", "", title.textContent?.trim() ?? ""));
  name.setAttribute("role", "heading");
  name.setAttribute("aria-level", "2");
  head.appendChild(name);
  if (done && !parsed) name.appendChild(goal());
  if (intro.length) {
    const text = element("div", "npa-next-intro");
    for (const paragraph of intro) text.appendChild(copy(paragraph));
    head.appendChild(text);
  }
  section.prepend!(head);
  added.push(head);
  // A row left with only hidden blocks would still add its spacing.
  const own = message.closest("[data-message-id]");
  for (const part of parts) {
    const row = part.closest("[data-message-id]");
    if (row && row !== own && topLevel(part).every((node) => hidden.includes(node)))
      hidden.push(row);
  }
  const prior = hidden.map((node) => node.getAttribute("data-npa-folded"));
  for (const node of hidden) node.setAttribute("data-npa-folded", "true");
  // Virtualized rows unmount and remount; a node that left with its row needs no refold.
  const owners = hidden.map(
    (node) => parts.find((part) => part.contains?.(node) || node.contains?.(part)) ?? node,
  );
  return {
    undo() {
      for (const node of added) node.remove();
      hidden.forEach((node, position) => {
        if (prior[position] === null) node.removeAttribute("data-npa-folded");
        else node.setAttribute("data-npa-folded", prior[position]!);
      });
    },
    intact: () =>
      hidden.every((node, position) =>
        node.isConnected
          ? node.getAttribute("data-npa-folded") === "true"
          : !owners[position].isConnected,
      ) &&
      messageParts(message).every((part) => parts.includes(part)) &&
      messageParts(message)
        .flatMap(topLevel)
        .every((node) => contents.has(node) && contents.get(node) === text(node)),
  };
}

function ownText(node: Node, skip: Node): string {
  if (node === skip) return "";
  if (node.nodeType === 3) return node.nodeValue ?? "";
  if (node.nodeType !== 1) return "";
  return Array.from(node.childNodes ?? [])
    .map((child) => ownText(child, skip))
    .join("");
}
