import type { Candidate, Snapshot } from "../shared/contracts";
import { selectionAllowed } from "../shared/next-prompts";
import type { Document, Node } from "./web";

export function renderSelection(
  doc: Document,
  root: Node,
  footerRoot: Node,
  candidates: Candidate[],
  snapshot: Snapshot,
  actions: { edit(picked: Candidate[]): void; send(picked: Candidate[]): Promise<void> },
) {
  let current = candidates,
    latest = snapshot,
    pending = false;
  const selected = new Set<string>();
  const inputs = new Map<string, Node>();
  const { exclusiveGroups: groups, allowedCombinations: combos } = candidates[0].selection!;
  const list = doc.createElement("div");
  list.setAttribute("class", "npa-selection-list");
  root.appendChild(list);
  const containers = new Map<string, Node>();
  for (const group of layout(
    candidates.map((c) => c.selection!.id),
    groups,
    combos,
  )) {
    const fieldset = doc.createElement("fieldset");
    fieldset.setAttribute("class", "npa-choice-group");
    fieldset.setAttribute("data-group", group.kind);
    const legend = doc.createElement("legend");
    const hint = doc.createElement("span");
    hint.setAttribute("class", "npa-group-hint");
    [legend.textContent, hint.textContent] =
      group.kind === "choice"
        ? [
            groups.length === 1 ? "Choose one" : `Choice ${group.index + 1} · choose one`,
            "Pick at most one",
          ]
        : group.kind === "follow-up"
          ? ["Follow-up", "Can be added to the choice above"]
          : group.kind === "together"
            ? [
                group.set ? `Send together · set ${group.set}` : "Send together",
                "Pick one or send the set as one message",
              ]
            : ["Send alone", "Cannot be combined with other suggestions"];
    fieldset.appendChild(legend);
    fieldset.appendChild(hint);
    const options = doc.createElement("div");
    options.setAttribute("class", "npa-choice-options");
    fieldset.appendChild(options);
    list.appendChild(fieldset);
    for (const id of group.ids) containers.set(id, options);
  }
  for (const [index, candidate] of candidates.entries()) {
    const id = candidate.selection!.id;
    const groupIndex = groups.findIndex((group) => group.includes(id));
    const container = containers.get(id)!;
    const row = doc.createElement("label");
    row.setAttribute("class", "npa-choice");
    const input = doc.createElement("input");
    input.setAttribute("type", groupIndex < 0 ? "checkbox" : "radio");
    if (groupIndex >= 0)
      input.setAttribute("name", `npa-${candidate.selection!.blockKey}-${groupIndex}`);
    input.setAttribute("aria-label", candidate.text);
    input.setAttribute("value", id);
    input.checked = false;
    input.addEventListener("change", () => {
      if (input.disabled || pending) {
        refresh();
        return;
      }
      if (input.checked) {
        if (groupIndex >= 0)
          for (const other of current)
            if (groups[groupIndex].includes(other.selection!.id)) selected.delete(other.key);
        selected.add(candidate.key);
      } else selected.delete(candidate.key);
      refresh();
    });
    inputs.set(candidate.key, input);
    row.appendChild(input);
    const mark = doc.createElement("span");
    mark.setAttribute("class", "npa-mark");
    mark.setAttribute("aria-hidden", "true");
    row.appendChild(mark);
    const label = doc.createElement("span");
    label.setAttribute("class", "npa-label");
    const text = doc.createElement("span");
    text.textContent = candidate.text;
    label.appendChild(text);
    if (candidate.why) {
      const why = doc.createElement("span");
      why.setAttribute("class", "npa-why");
      candidate.why.split(/\*\*(.+?)\*\*/).forEach((part, partIndex) => {
        if (!part) return;
        const span = doc.createElement(partIndex % 2 ? "strong" : "span");
        span.textContent = part;
        why.appendChild(span);
      });
      label.appendChild(why);
    }
    row.appendChild(label);
    const state = doc.createElement("span");
    state.setAttribute("class", "npa-choice-state");
    state.setAttribute("data-choice-index", String(index));
    row.appendChild(state);
    container.appendChild(row);
  }
  const footer = doc.createElement("div");
  footer.setAttribute("class", "npa-selection-footer");
  const summary = doc.createElement("div");
  summary.setAttribute("class", "npa-selection-summary");
  summary.setAttribute("role", "status");
  const count = doc.createElement("strong");
  const detail = doc.createElement("span");
  summary.appendChild(count);
  summary.appendChild(detail);
  footer.appendChild(summary);
  const buttons = doc.createElement("div");
  buttons.setAttribute("class", "npa-actions");
  const button = (className: string, text: string) => {
    const node = doc.createElement("button");
    node.setAttribute("class", className);
    node.setAttribute("type", "button");
    node.textContent = text;
    buttons.appendChild(node);
    return node;
  };
  const clear = button("npa-selection-clear", "Clear");
  clear.setAttribute("aria-label", "Clear selection");
  const edit = button("npa-edit npa-selection-edit", "Edit selected");
  const send = button("npa-send npa-selection-send", "Send selected");
  const picked = () => current.filter((c) => selected.has(c.key));
  const ready = () =>
    !pending &&
    !latest.busy &&
    latest.note !== "Jev reviewing..." &&
    selectionAllowed(picked()) &&
    picked().every((c) => c.state === "ready");
  function refresh() {
    const chosen = picked();
    const allowed = selectionAllowed(chosen);
    count.textContent = `${chosen.length} selected`;
    detail.textContent =
      chosen.length > 1
        ? allowed
          ? "One numbered message"
          : "Add or remove a suggestion to match a set"
        : chosen.length
          ? "One prompt"
          : "Choose a suggestion to continue";
    summary.setAttribute("data-invalid", String(chosen.length > 1 && !allowed));
    const ids = chosen.map((c) => c.selection!.id);
    for (const [index, candidate] of current.entries()) {
      const input = inputs.get(candidate.key)!;
      input.checked = selected.has(candidate.key);
      const id = candidate.selection!.id;
      // Suggestions that no declared combination can join with the selection are locked.
      const blocked = !input.checked && !reachable(ids, id, groups, combos);
      input.disabled =
        blocked ||
        pending ||
        latest.busy ||
        candidate.state !== "ready" ||
        latest.note === "Jev reviewing...";
      input.parentElement!.setAttribute("data-selected", String(input.checked));
      input.parentElement!.setAttribute("data-blocked", String(blocked));
      root.querySelector(`[data-choice-index="${index}"]`)!.textContent =
        candidate.state === "sent"
          ? "Sent"
          : candidate.state === "unknown"
            ? "Check chat"
            : candidate.state === "sending"
              ? "Sending..."
              : !blocked
                ? ""
                : combos.some((combo) => combo.includes(id))
                  ? "Not with selection"
                  : "Send alone";
    }
    clear.disabled = pending || !selected.size;
    edit.disabled = send.disabled = !ready();
    send.textContent = pending
      ? "Sending..."
      : chosen.some((c) => c.state === "unknown")
        ? "Check chat"
        : chosen.length && chosen.every((c) => c.state === "sent")
          ? "Sent"
          : chosen.some((c) => c.state === "sending")
            ? "Sending..."
            : chosen.length
              ? `Send selected (${chosen.length})`
              : "Send selected";
  }
  clear.addEventListener("click", () => {
    if (clear.disabled) return;
    selected.clear();
    refresh();
  });
  edit.addEventListener("click", () => {
    if (ready()) actions.edit(picked());
  });
  send.addEventListener("click", () => {
    if (!ready()) return;
    const chosen = picked();
    pending = true;
    refresh();
    void actions.send(chosen).finally(() => {
      pending = false;
      refresh();
    });
  });
  footer.appendChild(buttons);
  footerRoot.appendChild(footer);
  refresh();
  return (next: Candidate[], snapshot: Snapshot) => {
    current = next;
    latest = snapshot;
    refresh();
  };
}

type Group =
  | { kind: "choice"; index: number; ids: string[] }
  | { kind: "together"; set: number; ids: string[] }
  | { kind: "follow-up" | "alone"; ids: string[] };

// Exclusive groups become radio groups. Other suggestions linked by allowed combinations share a
// group: "follow-up" when a linked combination includes an exclusive choice, else "together".
// Suggestions in no combination are grouped as "alone".
export function layout(order: string[], groups: string[][], combos: string[][]): Group[] {
  const exclusive = new Set(groups.flat());
  const result: Group[] = groups.map((ids, index) => ({ kind: "choice", index, ids }));
  const seen = new Set<string>();
  const together: { set: number }[] = [];
  for (const id of order) {
    if (exclusive.has(id) || seen.has(id) || !combos.some((c) => c.includes(id))) continue;
    const ids = [id];
    seen.add(id);
    for (let i = 0; i < ids.length; i++)
      for (const combo of combos)
        if (combo.includes(ids[i]))
          for (const other of combo)
            if (!exclusive.has(other) && !seen.has(other)) {
              seen.add(other);
              ids.push(other);
            }
    const follow = combos.some(
      (c) => c.some((x) => ids.includes(x)) && c.some((x) => exclusive.has(x)),
    );
    if (follow) result.push({ kind: "follow-up", ids });
    else {
      const group = { kind: "together" as const, set: 0, ids };
      together.push(group);
      result.push(group);
    }
  }
  if (together.length > 1) together.forEach((group, index) => (group.set = index + 1));
  const alone = order.filter((id) => !exclusive.has(id) && !combos.some((c) => c.includes(id)));
  if (alone.length) result.push({ kind: "alone", ids: alone });
  return result;
}

// A radio replaces its exclusive group's selection; the result must fit one declared combination.
export function reachable(
  selected: string[],
  id: string,
  groups: string[][],
  combos: string[][],
): boolean {
  const group = groups.find((g) => g.includes(id)) ?? [];
  const next = [...selected.filter((x) => !group.includes(x)), id];
  return next.length === 1 || combos.some((combo) => next.every((x) => combo.includes(x)));
}
