import type { Candidate, Scope, Snapshot } from "../shared/contracts";
import { gitAction, joinPrompts, parsePrompts } from "../shared/prompts";
import { parseNextPrompts } from "../shared/next-prompts";
import { renderSelection } from "./selection";
import { decorateRecap, foldPanel, messageParts, recapStyles, underNextHeading } from "./recap";

export interface Node {
  textContent: string | null;
  isConnected: boolean;
  parentElement: Node | null;
  disabled: boolean;
  checked?: boolean;
  title: string;
  style: {
    cssText: string;
    setProperty(key: string, value: string): void;
    removeProperty?(key: string): void;
  };
  getBoundingClientRect?(): { width: number };
  querySelector(selector: string): Node | null;
  querySelectorAll(selector: string): ArrayLike<Node>;
  closest(selector: string): Node | null;
  contains?(node: Node): boolean;
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
  appendChild(node: Node): void;
  remove(): void;
  addEventListener(
    name: string,
    handler: (event: { key?: string; metaKey?: boolean }) => void,
  ): void;
  removeEventListener?(name: string, handler: (event: { metaKey?: boolean }) => void): void;
  value?: string;
  focus?(): void;
  setSelectionRange?(start: number, end: number): void;
  dispatchEvent(event: object): void;
  cloneNode?(deep: boolean): Node;
  childNodes?: ArrayLike<Node>;
  nodeType?: number;
  nodeValue?: string | null;
  prepend?(node: Node): void;
}
export interface Document extends Node {
  head: Node;
  body: Node;
  createElement(tag: string): Node;
  defaultView?: {
    Event: new (type: string, init?: { bubbles?: boolean }) => object;
    CustomEvent?: new (type: string, init: { detail: { serverId: string } }) => object;
  } | null;
}
interface Fiber {
  memoizedProps?: Record<string, unknown>;
  return?: Fiber;
  alternate?: Fiber;
  stateNode?: { current?: Fiber };
}
declare const document: Document;
declare const navigator: { userAgent: string };
type Mutation = { target: Node; addedNodes: ArrayLike<Node>; removedNodes: ArrayLike<Node> };
declare const MutationObserver: new (callback: (records: Mutation[]) => void) => {
  observe(node: Node, options: unknown): void;
  disconnect(): void;
};
declare function getComputedStyle(node: Node): { color: string; backgroundColor: string };

export type Binding = Scope & { message: string; timestamp: number };
export function binding(node: Node): Binding | null {
  let message: string | undefined, timestamp: number | undefined;
  let scope: Scope | undefined;
  const fiberKey = Object.keys(node).find((key) => key.startsWith("__reactFiber$"));
  if (!fiberKey) return null;
  let fiber = (node as unknown as Record<string, Fiber>)[fiberKey];
  // React keeps the DOM pointer on either buffer across commits. Read the active tree.
  let root = fiber;
  for (let depth = 0; root?.return && depth < 200; depth++) root = root.return;
  if (root?.stateNode?.current && root.stateNode.current !== root) {
    if (!fiber.alternate) return null;
    fiber = fiber.alternate;
  }
  for (let depth = 0; fiber && depth < 100; depth++, fiber = fiber.return!) {
    const p = fiber.memoizedProps;
    if (!p) continue;
    if (
      typeof p.message === "string" &&
      typeof p.timestamp === "number" &&
      typeof p.phase === "string"
    ) {
      if (p.phase !== "complete") return null;
      message ??= p.message;
      timestamp ??= p.timestamp;
    }
    if (
      typeof p.agentId === "string" &&
      typeof p.serverId === "string" &&
      typeof p.workspaceId === "string"
    ) {
      scope = { agentId: p.agentId, serverId: p.serverId, workspaceId: p.workspaceId };
      break;
    }
  }
  return scope && message !== undefined && timestamp !== undefined
    ? { ...scope, message, timestamp }
    : null;
}
// Only the reply after the latest user message can offer actions. Its rendered timestamp is not
// compared exactly: a streamed reply keeps its last live chunk time, unlike the stored row.
function answers(candidate: Candidate, timestamp: number) {
  return candidate.after !== undefined && timestamp > candidate.after;
}
export function desktopSupported() {
  return (
    typeof document !== "undefined" &&
    typeof navigator !== "undefined" &&
    /Electron\//.test(navigator.userAgent)
  );
}

type Controller = {
  inspect(scope: Scope): Promise<Snapshot>;
  send(scope: Scope, key: string | string[]): Promise<{ sent: boolean }>;
  /** Starts an unrelated suggestion in a new conversation. Without it, those rows stay read-only. */
  start?(scope: Scope, key: string): Promise<{ started: boolean }>;
  /** Runs on click with the pending outcome, so navigation need not wait for the send. */
  sending?(outcome: Promise<boolean>, scope: Scope): void;
};
// The Board plugin listens for these events; plugin surfaces cannot open another plugin's surface.
export type BoardEvent = "paseo-board:open" | "paseo-board:sent" | "paseo-board:send-failed";
export function boardEvent(name: BoardEvent, serverId: string, doc: Document = document) {
  const view = doc.defaultView;
  if (view?.CustomEvent)
    doc.dispatchEvent(
      new view.CustomEvent(name.replace("paseo-board:", "paseo-board:v2:"), {
        detail: { serverId },
      }),
    );
}
const OWNER = "data-next-prompt-actions";
// Phosphor Icons regular 2.1.1 (MIT), one family per TASTE.md.
const icons = {
  recap:
    '<path d="M136,80v43.47l36.12,21.67a8,8,0,0,1-8.24,13.72l-40-24A8,8,0,0,1,120,128V80a8,8,0,0,1,16,0Zm-8-48A95.44,95.44,0,0,0,60.08,60.15C52.81,67.51,46.35,74.59,40,82V64a8,8,0,0,0-16,0v40a8,8,0,0,0,8,8H72a8,8,0,0,0,0-16H49c7.15-8.42,14.27-16.35,22.39-24.57a80,80,0,1,1,1.66,114.75,8,8,0,1,0-11,11.64A96,96,0,1,0,128,32Z"/>', // clock-counter-clockwise
  next: '<path d="M229.66,157.66l-48,48a8,8,0,0,1-11.32-11.32L204.69,160H128A104.11,104.11,0,0,1,24,56a8,8,0,0,1,16,0,88.1,88.1,0,0,0,88,88h76.69l-34.35-34.34a8,8,0,0,1,11.32-11.32l48,48A8,8,0,0,1,229.66,157.66Z"/>', // arrow-bend-down-right
  branch:
    '<path d="M232,64a32,32,0,1,0-40,31v17a8,8,0,0,1-8,8H96a23.84,23.84,0,0,0-8,1.38V95a32,32,0,1,0-16,0v66a32,32,0,1,0,16,0V144a8,8,0,0,1,8-8h88a24,24,0,0,0,24-24V95A32.06,32.06,0,0,0,232,64ZM64,64A16,16,0,1,1,80,80,16,16,0,0,1,64,64ZM96,192a16,16,0,1,1-16-16A16,16,0,0,1,96,192ZM200,80a16,16,0,1,1,16-16A16,16,0,0,1,200,80Z"/>', // git-branch
  commit:
    '<path d="M248,120H183.42a56,56,0,0,0-110.84,0H8a8,8,0,0,0,0,16H72.58a56,56,0,0,0,110.84,0H248a8,8,0,0,0,0-16ZM128,168a40,40,0,1,1,40-40A40,40,0,0,1,128,168Z"/>', // git-commit
  done: '<path d="M173.66,98.34a8,8,0,0,1,0,11.32l-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35A8,8,0,0,1,173.66,98.34ZM232,128A104,104,0,1,1,128,24,104.11,104.11,0,0,1,232,128Zm-16,0a88,88,0,1,0-88,88A88.1,88.1,0,0,0,216,128Z"/>', // check-circle
  edit: '<path d="M227.31,73.37,182.63,28.68a16,16,0,0,0-22.63,0L36.69,152A15.86,15.86,0,0,0,32,163.31V208a16,16,0,0,0,16,16H92.69A15.86,15.86,0,0,0,104,219.31L227.31,96a16,16,0,0,0,0-22.63ZM92.69,208H48V163.31l88-88L180.69,120ZM192,108.68,147.31,64l24-24L216,84.68Z"/>', // pencil-simple
  send: '<path d="M205.66,117.66a8,8,0,0,1-11.32,0L136,59.31V216a8,8,0,0,1-16,0V59.31L61.66,117.66a8,8,0,0,1-11.32-11.32l72-72a8,8,0,0,1,11.32,0l72,72A8,8,0,0,1,205.66,117.66Z"/>', // arrow-up
  thread:
    '<path d="M224,128a8,8,0,0,1-8,8H136v80a8,8,0,0,1-16,0V136H40a8,8,0,0,1,0-16h80V40a8,8,0,0,1,16,0v80h80A8,8,0,0,1,224,128Z"/>', // plus
  push: '<path d="M224,144v64a8,8,0,0,1-8,8H40a8,8,0,0,1-8-8V144a8,8,0,0,1,16,0v56H208V144a8,8,0,0,1,16,0ZM93.66,77.66,120,51.31V144a8,8,0,0,0,16,0V51.31l26.34,26.35a8,8,0,0,0,11.32-11.32l-40-40a8,8,0,0,0-11.32,0l-40,40A8,8,0,0,0,93.66,77.66Z"/>', // upload-simple
};
function iconMask(shape: string) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="black">${shape}</svg>`;
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
}
// Panel follows docs/designs/recap-next-panel-2026-09-25/TASTE.md: neutral surfaces from the host
// ink, one accent for the What next area, radii 12/8/6. The fence box, raw `prompt:`/`why:` text,
// and its copy button hide behind the panel.
const styles = `
[${OWNER}] {--npa-accent:#0f7b5f;--npa-line:color-mix(in srgb,var(--npa-ink,currentColor) 11%,transparent);--npa-line-strong:color-mix(in srgb,var(--npa-ink,currentColor) 20%,transparent);--npa-surface:color-mix(in srgb,var(--npa-ink,currentColor) 2.5%,var(--npa-paper,transparent));--npa-hover:color-mix(in srgb,var(--npa-ink,currentColor) 4%,transparent);--npa-chip:color-mix(in srgb,var(--npa-ink,currentColor) 6%,transparent);--npa-muted:color-mix(in srgb,var(--npa-ink,currentColor) 60%,transparent);--npa-soft:color-mix(in srgb,var(--npa-ink,currentColor) 80%,transparent);--npa-mono:"SF Mono",ui-monospace,Menlo,monospace;
  display:flex;flex-direction:column;margin:0;font:15px/1.6 -apple-system,"SF Pro Text",system-ui,sans-serif;color:var(--npa-ink,inherit);white-space:normal;text-align:left;border:1px solid var(--npa-line);border-radius:12px;background:var(--npa-paper,transparent);box-shadow:0 1px 2px rgb(24 24 27 / .04),0 10px 28px -16px rgb(24 24 27 / .14);overflow:hidden;}
[${OWNER}][data-npa-theme="dark"] {--npa-accent:#3ccf9e;box-shadow:0 1px 2px rgb(0 0 0 / .3),0 12px 32px -18px rgb(0 0 0 / .6);}
[${OWNER}] {--npa-accent-soft:color-mix(in srgb,var(--npa-accent) 9%,transparent);}
[data-npa-block] {background:transparent!important;border-color:transparent!important;padding:0!important;}
[data-npa-block] > :not([${OWNER}]) {display:none!important;}
/* Desktop measured a 6px gap between sections that this sheet never sets (an older sheet of this
   plugin set gap:6px here). Pin container spacing so outside rules cannot open gaps. */
[data-npa-block] > [${OWNER}] {gap:0!important;margin:0!important;padding:0!important;}
[${OWNER}] .npa-section {padding:18px 20px 20px;min-width:0;}
[${OWNER}] .npa-section + .npa-section {border-top:1px solid var(--npa-line);}
[${OWNER}] .npa-recap {padding:14px 20px 16px;background:var(--npa-surface);}
[${OWNER}] .npa-recap-head {display:flex;align-items:center;justify-content:space-between;gap:8px 12px;flex-wrap:wrap;margin-bottom:6px;}
[${OWNER}] .npa-kicker {display:inline-flex;align-items:center;gap:8px;font-size:15px;font-weight:600;line-height:1.4;letter-spacing:-.005em;color:var(--npa-ink,inherit);}
[${OWNER}] .npa-badge {display:inline-grid;place-items:center;flex-shrink:0;}
[${OWNER}] .npa-badge::before, [${OWNER}] .npa-chip::before {content:"";width:15px;height:15px;flex-shrink:0;background:currentColor;mask:var(--npa-icon) center / contain no-repeat;}
[${OWNER}] .npa-recap .npa-badge {color:var(--npa-muted);--npa-icon:${iconMask(icons.recap)};}
[${OWNER}] .npa-next .npa-badge {color:var(--npa-accent);--npa-icon:${iconMask(icons.next)};}
[${OWNER}] .npa-badge::before {width:16px;height:16px;}
[${OWNER}] .npa-meta {display:flex;gap:6px;flex-wrap:wrap;}
[${OWNER}] .npa-chip {display:inline-flex;align-items:center;gap:5px;min-height:24px;padding:0 8px;border-radius:6px;background:var(--npa-chip);color:var(--npa-soft);font-size:12.5px;font-weight:500;line-height:1.3;white-space:nowrap;}
[${OWNER}] .npa-chip::before {width:13px;height:13px;background:var(--npa-muted);}
[${OWNER}] .npa-branch {font-family:var(--npa-mono);font-size:12px;font-weight:400;color:var(--npa-ink,inherit);--npa-icon:${iconMask(icons.branch)};}
[${OWNER}] .npa-commit {--npa-icon:${iconMask(icons.commit)};}
[${OWNER}] .npa-commit.npa-ok {--npa-icon:${iconMask(icons.done)};}
[${OWNER}] .npa-branch-value, [${OWNER}] .npa-commit-value {display:inline-block;}
[${OWNER}] .npa-commit-value::first-letter {text-transform:uppercase;}
[${OWNER}] .npa-did {margin:0;max-width:72ch;font-size:14px;line-height:1.6;color:var(--npa-soft);overflow-wrap:anywhere;text-wrap:pretty;}
[${OWNER}] .npa-did::first-letter {text-transform:uppercase;}
[${OWNER}] .npa-did *, [${OWNER}] .npa-next-intro * {display:inline!important;margin:0!important;font-size:inherit!important;line-height:inherit!important;}
[${OWNER}] .npa-next-intro > * {display:block!important;}
[${OWNER}] .npa-did [data-npa-list] {display:block!important;}
[${OWNER}] .npa-did [data-npa-list="li"] {display:flex!important;gap:8px;margin-top:2px!important;}
[${OWNER}] [data-npa-code] {font-family:var(--npa-mono)!important;font-size:.88em!important;padding:1px 5px!important;border-radius:6px;background:var(--npa-chip)!important;color:var(--npa-ink,inherit)!important;}
[${OWNER}] .npa-chip * {display:inline!important;margin:0!important;padding:0!important;background:none!important;font-size:inherit!important;line-height:inherit!important;}
[${OWNER}] .npa-next-head {margin-bottom:16px;}
[${OWNER}] .npa-next-title {display:flex;align-items:center;gap:8px;font-size:15px;font-weight:600;line-height:1.4;letter-spacing:-.005em;}
[${OWNER}] .npa-next-intro {margin:4px 0 0 24px;color:var(--npa-soft);max-width:68ch;overflow-wrap:anywhere;text-wrap:pretty;}
[${OWNER}] .npa-row {display:grid;grid-template-columns:minmax(0,1fr) auto;gap:14px 24px;align-items:end;padding:16px 16px 16px 18px;border:1px solid var(--npa-line);border-radius:8px;background:var(--npa-paper,transparent);transition:border-color .15s ease;}
[${OWNER}] .npa-row + .npa-row {margin-top:10px;}
[${OWNER}][data-npa-readonly] .npa-row, [${OWNER}][data-npa-readonly] .npa-row:hover {grid-template-columns:minmax(0,1fr);border-color:var(--npa-line);}
[${OWNER}] .npa-label {min-width:0;font-weight:400;white-space:pre-wrap;overflow-wrap:anywhere;text-wrap:pretty;}
[${OWNER}] .npa-why {display:block;margin-top:4px;font-size:13px;line-height:1.5;font-weight:400;white-space:normal;color:var(--npa-muted);}
[${OWNER}] .npa-why strong {font-weight:600;color:var(--npa-soft);}
[${OWNER}] .npa-actions {display:flex;align-items:center;justify-content:flex-end;flex-wrap:wrap;gap:8px;}
[${OWNER}] button {display:inline-flex;align-items:center;justify-content:center;gap:6px;flex-shrink:0;min-height:32px;padding:0 12px;border-radius:8px;border:1px solid var(--npa-line-strong);background:var(--npa-paper,transparent);color:var(--npa-ink,inherit);font-family:inherit;font-size:13px;font-weight:500;line-height:1;white-space:nowrap;cursor:pointer;user-select:none;transition:background-color .15s ease,border-color .15s ease,transform .1s ease;}
[${OWNER}] button::before {content:"";width:14px;height:14px;flex-shrink:0;background:currentColor;mask:var(--npa-icon) center / contain no-repeat;}
[${OWNER}] .npa-edit {--npa-icon:${iconMask(icons.edit)};}
[${OWNER}] .npa-send {--npa-icon:${iconMask(icons.send)};min-width:80px;background:var(--npa-ink,#18181b);color:var(--npa-paper,#fff);border-color:var(--npa-ink,#18181b);}
[${OWNER}] .npa-send.npa-commit {--npa-icon:${iconMask(icons.commit)};}
[${OWNER}] .npa-send.npa-push {--npa-icon:${iconMask(icons.push)};}
[${OWNER}] .npa-start, [${OWNER}] .npa-send[data-npa-thread] {--npa-icon:${iconMask(icons.thread)};}
[${OWNER}] .npa-start[data-npa-here] {--npa-icon:${iconMask(icons.send)};}
[${OWNER}] .npa-goal {--npa-icon:${iconMask(icons.done)};}
[${OWNER}] .npa-other-head {display:flex;align-items:baseline;justify-content:space-between;flex-wrap:wrap;gap:4px 12px;margin-bottom:12px;}
[${OWNER}] .npa-other-title {font-size:13px;font-weight:600;line-height:20px;}
[${OWNER}] .npa-other-hint {font-size:12.5px;line-height:20px;color:var(--npa-muted);}
[${OWNER}] .npa-selection-clear {border-color:transparent;background:transparent;color:var(--npa-muted);padding:0 8px;}
[${OWNER}] .npa-selection-clear::before {display:none;}
@media (hover:hover) {
  [${OWNER}] .npa-row:hover {border-color:var(--npa-line-strong);}
  [${OWNER}] .npa-edit:not(:disabled):hover, [${OWNER}] .npa-start:not(:disabled):hover {background:var(--npa-hover);}
  [${OWNER}] .npa-send:not(:disabled):hover {background:color-mix(in srgb,var(--npa-ink,#18181b) 86%,var(--npa-paper,#fff));}
  [${OWNER}] .npa-selection-clear:not(:disabled):hover {color:var(--npa-ink,inherit);}
  [${OWNER}] .npa-choice:not(:has(input:disabled)):hover {background:var(--npa-hover);}
}
[${OWNER}] button:not(:disabled):active {transform:scale(.98);}
[${OWNER}] button:focus-visible {outline:2px solid var(--npa-accent);outline-offset:2px;}
[${OWNER}] button:disabled {opacity:.4;cursor:default;}
[${OWNER}] .npa-note {margin-top:12px;font-size:13px;color:var(--npa-muted);overflow-wrap:anywhere;}
[${OWNER}] .npa-note:empty {display:none;}
[${OWNER}] .npa-selection-list {display:flex;flex-direction:column;gap:16px;}
[${OWNER}] .npa-choice-group {border:0;padding:0;margin:0;min-width:0;}
[${OWNER}] .npa-choice-group legend {float:left;padding:0;margin:0 12px 8px 0;font-size:13px;font-weight:600;line-height:20px;}
[${OWNER}] .npa-group-hint {display:block;margin-bottom:8px;text-align:right;font-size:12.5px;line-height:20px;color:var(--npa-muted);}
[${OWNER}] .npa-choice-options {clear:both;border:1px solid var(--npa-line);border-radius:8px;overflow:hidden;}
[${OWNER}] .npa-choice {position:relative;display:grid;grid-template-columns:18px minmax(0,1fr) auto;gap:12px;align-items:start;padding:12px 14px;cursor:pointer;transition:background-color .15s ease;}
[${OWNER}] .npa-choice + .npa-choice {border-top:1px solid var(--npa-line);}
[${OWNER}] .npa-choice[data-selected="true"] {background:var(--npa-accent-soft);}
[${OWNER}] .npa-choice:has(input:disabled) {cursor:default;}
[${OWNER}] .npa-choice input {position:absolute;width:1px;height:1px;margin:0;opacity:0;pointer-events:none;}
[${OWNER}] .npa-mark {display:grid;place-items:center;box-sizing:border-box;width:18px;height:18px;margin-top:3px;border:1.5px solid var(--npa-line-strong);border-radius:50%;background:var(--npa-paper,transparent);transition:border-color .15s ease,background-color .15s ease;}
[${OWNER}] input[type="checkbox"] + .npa-mark {border-radius:6px;}
[${OWNER}] input:checked + .npa-mark {border-color:var(--npa-accent);}
[${OWNER}] input[type="radio"]:checked + .npa-mark::after {content:"";width:8px;height:8px;border-radius:50%;background:var(--npa-accent);}
[${OWNER}] input[type="checkbox"]:checked + .npa-mark {background:var(--npa-accent);}
[${OWNER}] input[type="checkbox"]:checked + .npa-mark::after {content:"";width:9px;height:5px;border:2px solid var(--npa-paper,#fff);border-top:0;border-right:0;transform:translateY(-1px) rotate(-45deg);}
[${OWNER}] input:focus-visible + .npa-mark {outline:2px solid var(--npa-accent);outline-offset:2px;}
[${OWNER}] input:disabled + .npa-mark {opacity:.5;}
[${OWNER}] .npa-choice .npa-label {font-weight:400;}
[${OWNER}] .npa-choice-state {padding-top:3px;font-size:12px;color:var(--npa-muted);white-space:nowrap;}
[${OWNER}] .npa-choice-state:empty {display:none;}
[${OWNER}] .npa-choice[data-blocked="true"] .npa-label, [${OWNER}] .npa-choice[data-blocked="true"] .npa-mark {opacity:.45;}
[${OWNER}] .npa-selection-footer {display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:12px 16px;padding:12px 20px;border-top:1px solid var(--npa-line);background:var(--npa-surface);}
[${OWNER}] .npa-selection-summary {min-width:140px;font-size:13px;line-height:1.45;font-variant-numeric:tabular-nums;overflow-wrap:anywhere;}
[${OWNER}] .npa-selection-summary strong {display:block;font-weight:600;}
[${OWNER}] .npa-selection-summary span {display:block;font-size:12.5px;color:var(--npa-muted);}
[${OWNER}] .npa-selection-summary[data-invalid="true"] span {color:var(--npa-ink,inherit);font-weight:500;}
[data-npa-next-heading] {font-size:22px!important;line-height:1.3!important;margin-top:32px!important;margin-bottom:12px!important;padding-bottom:0!important;border-bottom-width:0!important;}
[data-npa-next-heading] * {font-size:inherit!important;line-height:inherit!important;border-bottom-width:0!important;}
@media (pointer:coarse) { [${OWNER}] button {min-height:44px;} }
@media (prefers-reduced-motion:reduce) {
  [${OWNER}] button, [${OWNER}] .npa-choice, [${OWNER}] .npa-row, [${OWNER}] .npa-mark {transition:none;}
  [${OWNER}] button:not(:disabled):active {transform:none;}
}
@media (max-width:600px) {
  [${OWNER}] .npa-section, [${OWNER}] .npa-recap {padding-left:16px;padding-right:16px;}
  [${OWNER}] .npa-selection-footer {padding:12px 16px;}
  [${OWNER}] .npa-row {grid-template-columns:minmax(0,1fr);}
  [${OWNER}] .npa-next-intro {margin-left:0;}
}
`;

// The host marks the composer field with dataSet={{composerInput:""}}; the root testID is the
// fallback for hosts that predate it.
const FIELDS = ["[data-composer-input]", '[data-testid="message-input-root"] textarea'];
// Several conversations can stay mounted at once, so prefer the composer sharing the closest
// ancestor with the clicked prompt rather than whichever comes first in the document.
export function composerField(doc: Document, block?: Node): Node | null {
  const fields = FIELDS.flatMap((selector) => Array.from(doc.querySelectorAll(selector)));
  if (fields.length < 2) return fields[0] ?? null;
  for (let node = block ?? null; node; node = node.parentElement)
    for (const field of fields) if (node.contains?.(field)) return field;
  return fields[fields.length - 1];
}
export function fillComposer(text: string, doc: Document, block?: Node): string {
  const field = composerField(doc, block);
  const view = doc.defaultView;
  if (!field || !view) return "Composer not found. Open the conversation, then try again.";
  // The field is uncontrolled, but React still tracks the last value it saw: write through the
  // prototype setter and replay the input event so the host adopts the new text.
  const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), "value")?.set;
  if (setter) setter.call(field, text);
  else field.value = text;
  field.dispatchEvent(new view.Event("input", { bubbles: true }));
  field.focus?.();
  field.setSelectionRange?.(text.length, text.length);
  return field.value === text
    ? "Ready in the composer."
    : "The composer did not accept the text; copy the prompt text above.";
}

export function install(controller: Controller, doc: Document = document, identify = binding) {
  let stopped = false,
    scheduled = false,
    running = false,
    rescan = false,
    fresh = true;
  // A streaming reply mutates the DOM constantly. Reuse the last timeline read unless a prompt
  // block appeared or changed; the interval and user actions still force a fresh read.
  const cache = new Map<string, { seen: Set<string>; snapshot: Promise<Snapshot> }>();
  let scanTimer: ReturnType<typeof setTimeout> | undefined;
  type Owned = {
    structure: string;
    state: string;
    update(candidates: Candidate[], snapshot: Snapshot): void;
    intact(): boolean;
    rerender(): void;
    cleanup(): void;
  };
  const idle: Snapshot = { enabled: false, busy: false, note: "", candidates: [] };
  const owned = new Map<Node, Owned>();
  const recaps = new Map<Node, { message: string; timestamp: number; cleanup(): void }>();
  const headings = new Map<Node, string | null>();
  const style = doc.createElement("style");
  style.textContent = styles + recapStyles;
  doc.head.appendChild(style);
  // Holding Cmd over Send or Start in new thread swaps it to the other action until Cmd or the
  // pointer leaves.
  let hovered: ((meta: boolean) => void) | null = null;
  const onKey = (event: { key?: string; metaKey?: boolean }) => {
    if (event.key === "Meta") hovered?.(!!event.metaKey);
  };
  doc.addEventListener("keydown", onKey);
  doc.addEventListener("keyup", onKey);
  function setColors(ui: Node, block: Node) {
    if (typeof getComputedStyle !== "function") return;
    const ink = getComputedStyle(block).color;
    ui.style.setProperty("--npa-ink", ink);
    // Light ink means a dark host theme; the accent needs its lighter variant there.
    const [red = 0, green = 0, blue = 0] = (ink.match(/[\d.]+/g) ?? []).map(Number);
    ui.setAttribute("data-npa-theme", red + green + blue > 382 ? "dark" : "light");
    // Skip the fence itself: its code-block surface is hidden behind the panel.
    for (let parent = block.parentElement; parent; parent = parent.parentElement) {
      const background = getComputedStyle(parent).backgroundColor;
      if (background && background !== "transparent" && background !== "rgba(0, 0, 0, 0)") {
        ui.style.setProperty("--npa-paper", background);
        break;
      }
    }
  }
  function clear(block: Node) {
    owned.get(block)?.cleanup();
    owned.delete(block);
  }
  function valid(block: Node, expected: Binding, candidate: Candidate) {
    const now = identify(block.closest('[data-testid="assistant-message"]') ?? block);
    return (
      block.isConnected &&
      now &&
      now.agentId === expected.agentId &&
      now.serverId === expected.serverId &&
      now.workspaceId === expected.workspaceId &&
      now.message === expected.message &&
      now.timestamp === expected.timestamp &&
      answers(candidate, now.timestamp)
    );
  }
  // Earlier replies are no longer sendable; keep their panel for reading, without actions.
  function readOnly(block: Node, assistant: Node, context: Binding, code: string): Candidate[] {
    if (!underNextHeading(assistant, block)) return [];
    const info = code.trimStart().startsWith("{") ? "next-prompts" : "text";
    const [parsed] = parsePrompts(`## What Next\n~~~~${info}\n${code}\n~~~~`);
    return (parsed?.prompts ?? []).map((text, index) => ({
      key: "",
      block: code,
      text,
      why: parsed.whys[index] || undefined,
      ...(parsed.threads[index] ? { thread: "new" as const } : {}),
      ...(parsed.declaration?.goal ? { goal: parsed.declaration.goal } : {}),
      source: context.message,
      timestamp: context.timestamp,
      state: "sent",
    }));
  }
  function promptLabel(candidate: Candidate) {
    const label = doc.createElement("span");
    label.setAttribute("class", "npa-label");
    label.textContent = candidate.text;
    if (candidate.why) {
      const why = doc.createElement("span");
      why.setAttribute("class", "npa-why");
      // `**word**` marks the key outcome; odd split parts are the bold ones.
      candidate.why.split(/\*\*(.+?)\*\*/).forEach((part, index) => {
        if (!part) return;
        const span = doc.createElement(index % 2 ? "strong" : "span");
        span.textContent = part;
        why.appendChild(span);
      });
      label.appendChild(why);
    }
    return label;
  }
  function render(
    block: Node,
    context: Binding,
    candidates: Candidate[],
    snapshot: Snapshot,
    readonly = false,
  ) {
    // Rebuilding the controls flashes and shifts layout, so state changes patch them in place.
    const structure = JSON.stringify([
      context,
      readonly,
      candidates.map(({ state: _, ...rest }) => rest),
    ]);
    const state = JSON.stringify([
      candidates.map((c) => c.state),
      snapshot.enabled,
      snapshot.busy,
      snapshot.note,
      snapshot.warning,
    ]);
    const current = owned.get(block);
    if (current?.structure === structure && current.intact()) {
      const ui = block.querySelector(`[${OWNER}]`);
      if (ui) setColors(ui, block);
      if (current.state !== state) {
        current.state = state;
        current.update(candidates, snapshot);
      }
      return;
    }
    clear(block);
    if (block.querySelector(`[${OWNER}]`)) return;
    const ui = doc.createElement("div");
    ui.setAttribute(OWNER, "true");
    ui.setAttribute("data-paseo-markdown-ignore", "true");
    setColors(ui, block);
    const note = doc.createElement("div");
    note.setAttribute("class", "npa-note");
    note.setAttribute("role", "status");
    note.textContent = [snapshot.note, snapshot.warning].filter(Boolean).join(" ");
    const section = doc.createElement("div");
    section.setAttribute("class", "npa-section npa-next");
    ui.appendChild(section);
    const edits: Node[] = [];
    const sends: Node[] = [];
    const starts: Node[] = [];
    // Unrelated suggestions get their own section and can only start a new thread.
    const local = candidates.filter((c) => !c.thread);
    const others = candidates.filter((c) => c.thread);
    const gitActions = local.map((candidate) => gitAction(candidate.text));
    function mount(update: Owned["update"]) {
      section.appendChild(note);
      block.appendChild(ui);
      const message = block.closest('[data-testid="assistant-message"]');
      const done = candidates.some((c) => c.goal === "done");
      const fold = message ? foldPanel(doc, message, block, ui, section, done) : null;
      const prior = block.getAttribute("data-npa-block");
      block.setAttribute("data-npa-block", candidates.length === 1 ? "single" : "multiple");
      owned.set(block, {
        structure,
        state,
        update,
        intact: () => fold?.intact() ?? true,
        rerender: () => render(block, context, candidates, snapshot, readonly),
        cleanup() {
          fold?.undo();
          ui.remove();
          if (prior === null) block.removeAttribute("data-npa-block");
          else block.setAttribute("data-npa-block", prior);
        },
      });
    }
    // Cmd flips a Send and a Start button to the other action, which then needs its own guards.
    let blocked = snapshot.busy || snapshot.note === "Jev reviewing...";
    function flipOnCmd(button: Node, flip: (on: boolean) => void) {
      const hover = (event: { metaKey?: boolean }) => {
        hovered = flip;
        flip(!!event.metaKey);
      };
      button.addEventListener("mouseenter", (event) => {
        // A shorter label pulls the edge out from under the pointer, which leaves, flips back,
        // grows and enters again. Hold the entry width until the pointer leaves.
        const width = button.getBoundingClientRect?.().width;
        if (width) button.style.setProperty("min-width", `${width}px`);
        hover(event);
      });
      button.addEventListener("mousemove", hover);
      button.addEventListener("mouseleave", () => {
        if (hovered === flip) hovered = null;
        flip(false);
        button.style.removeProperty?.("min-width");
      });
    }
    function sendHere(key: string | string[]) {
      return action(async () => {
        const outcome = controller.send(context, key);
        controller.sending?.(
          outcome.then((r) => r.sent).catch(() => false),
          context,
        );
        await outcome;
      }, "Sending...");
    }
    function updateStarts(next: Candidate[], latest: Snapshot) {
      blocked = latest.busy || latest.note === "Jev reviewing...";
      next
        .filter((c) => c.thread)
        .forEach((candidate, index) => {
          if (!starts[index]) return;
          starts[index].textContent =
            candidate.state === "sent"
              ? "Started"
              : candidate.state === "unknown"
                ? "Check threads"
                : candidate.state === "sending"
                  ? "Starting..."
                  : starts[index].getAttribute("data-npa-here") !== null && !blocked
                    ? "Send"
                    : "Start in new thread";
          // A new thread never waits for this conversation to become idle.
          starts[index].disabled = candidate.state !== "ready";
        });
    }
    function renderOthers() {
      if (!others.length) return;
      const other = doc.createElement("div");
      other.setAttribute("class", "npa-section npa-other");
      other.setAttribute("role", "group");
      other.setAttribute("aria-label", "Other work");
      const head = doc.createElement("div");
      head.setAttribute("class", "npa-other-head");
      const title = doc.createElement("span");
      title.setAttribute("class", "npa-other-title");
      title.textContent = "Other work";
      const hint = doc.createElement("span");
      hint.setAttribute("class", "npa-other-hint");
      hint.textContent = "Not part of this task. Starts a separate thread.";
      head.appendChild(title);
      head.appendChild(hint);
      other.appendChild(head);
      for (const candidate of others) {
        const row = doc.createElement("div");
        row.setAttribute("class", "npa-row");
        row.appendChild(promptLabel(candidate));
        if (!readonly && controller.start) {
          const actions = doc.createElement("div");
          actions.setAttribute("class", "npa-actions");
          const start = doc.createElement("button");
          start.setAttribute("type", "button");
          start.setAttribute("class", "npa-start");
          start.setAttribute("aria-label", `Start in new thread: ${candidate.text}`);
          start.textContent = "Start in new thread";
          const sendable = () => !blocked && !gitAction(candidate.text);
          const hereMode = (on: boolean) => {
            on &&= sendable() && !start.disabled;
            if (on === (start.getAttribute("data-npa-here") !== null)) return;
            if (on) start.setAttribute("data-npa-here", "true");
            else start.removeAttribute("data-npa-here");
            start.setAttribute(
              "aria-label",
              `${on ? "Send suggested prompt" : "Start in new thread"}: ${candidate.text}`,
            );
            if (!start.disabled) start.textContent = on ? "Send" : "Start in new thread";
          };
          flipOnCmd(start, hereMode);
          start.addEventListener("click", (event) => {
            if (start.disabled || !valid(block, context, candidate)) return;
            if (event.metaKey && sendable()) {
              hereMode(false);
              start.textContent = "Sending...";
              void sendHere(candidate.key);
              return;
            }
            start.textContent = "Starting...";
            void action(() => controller.start!(context, candidate.key), "Starting new thread...");
          });
          starts.push(start);
          actions.appendChild(start);
          row.appendChild(actions);
        }
        other.appendChild(row);
      }
      ui.appendChild(other);
    }
    function update(all: Candidate[], latest: Snapshot) {
      note.textContent = [latest.note, latest.warning].filter(Boolean).join(" ");
      updateStarts(all, latest);
      all
        .filter((c) => !c.thread)
        .forEach((candidate, index) => {
          edits[index].disabled = false;
          sends[index].disabled =
            latest.busy || candidate.state !== "ready" || latest.note === "Jev reviewing...";
          sends[index].textContent =
            candidate.state === "sent"
              ? "Sent"
              : candidate.state === "unknown"
                ? "Check chat"
                : candidate.state === "sending"
                  ? "Sending..."
                  : sends[index].getAttribute("data-npa-thread") !== null && !sends[index].disabled
                    ? "Start in new thread"
                    : (gitActions[index]?.label ?? "Send");
        });
    }
    async function action(run: () => Promise<unknown>, label: string) {
      [...edits, ...sends, ...starts].forEach((button) => {
        button.disabled = true;
      });
      note.textContent = label;
      try {
        await run();
      } catch {
        note.textContent = "Action failed. Refresh state and try again.";
      } finally {
        const item = owned.get(block);
        if (item) item.state = "";
        fresh = true;
        schedule();
      }
    }
    if (readonly) {
      ui.setAttribute("data-npa-readonly", "true");
      for (const candidate of local) {
        const row = doc.createElement("div");
        row.setAttribute("class", "npa-row");
        row.appendChild(promptLabel(candidate));
        section.appendChild(row);
      }
      renderOthers();
      mount(() => {});
      return;
    }
    if (local.length > 1 && local.every((c) => c.selection) && !gitActions.some(Boolean)) {
      const update = renderSelection(doc, section, ui, local, snapshot, {
        edit(picked) {
          if (!picked.every((c) => valid(block, context, c))) return;
          note.textContent = fillComposer(joinPrompts(picked.map((c) => c.text)), doc, block);
        },
        async send(picked) {
          if (!picked.every((c) => valid(block, context, c))) return;
          const keys = picked.map((c) => c.key);
          await sendHere(keys.length === 1 ? keys[0] : keys);
        },
      });
      renderOthers();
      updateStarts(candidates, snapshot);
      mount((next, latest) => {
        note.textContent = [latest.note, latest.warning].filter(Boolean).join(" ");
        updateStarts(next, latest);
        update(
          next.filter((c) => !c.thread),
          latest,
        );
      });
      return;
    }
    for (const [index, candidate] of local.entries()) {
      const git = gitActions[index];
      const row = doc.createElement("div");
      row.setAttribute("class", "npa-row");
      row.appendChild(promptLabel(candidate));
      const actions = doc.createElement("div");
      actions.setAttribute("class", "npa-actions");
      const edit = doc.createElement("button");
      edit.setAttribute("type", "button");
      edit.setAttribute("class", "npa-edit");
      edit.setAttribute("aria-label", `Edit suggested prompt in the composer: ${candidate.text}`);
      edit.textContent = "Edit";
      edit.addEventListener("click", () => {
        if (edit.disabled || !valid(block, context, candidate)) return;
        note.textContent = fillComposer(candidate.text, doc, block);
      });
      edits.push(edit);
      actions.appendChild(edit);
      const send = doc.createElement("button");
      send.setAttribute("type", "button");
      send.setAttribute(
        "class",
        git ? `npa-send npa-${git.kind === "push" ? "push" : "commit"}` : "npa-send",
      );
      const label = `${git?.label ?? "Send"} suggested prompt: ${candidate.text}`;
      send.setAttribute("aria-label", label);
      // Git actions need this conversation's work, so they never start a new thread.
      const threadable = !git && !!controller.start;
      const threadMode = (on: boolean) => {
        on &&= threadable && !send.disabled;
        if (on === (send.getAttribute("data-npa-thread") !== null)) return;
        if (on) send.setAttribute("data-npa-thread", "true");
        else send.removeAttribute("data-npa-thread");
        send.setAttribute("aria-label", on ? `Start in new thread: ${candidate.text}` : label);
        if (!send.disabled) send.textContent = on ? "Start in new thread" : "Send";
      };
      if (threadable) flipOnCmd(send, threadMode);
      send.addEventListener("click", (event) => {
        if (send.disabled || !valid(block, context, candidate)) return;
        if (threadable && event.metaKey) {
          threadMode(false);
          send.textContent = "Starting...";
          void action(() => controller.start!(context, candidate.key), "Starting new thread...");
          return;
        }
        send.textContent = "Sending...";
        void sendHere(candidate.key);
      });
      sends.push(send);
      actions.appendChild(send);
      row.appendChild(actions);
      section.appendChild(row);
    }
    renderOthers();
    update(candidates, snapshot);
    mount(update);
  }
  function decorate(assistant: Node, context: Binding) {
    const previous = recaps.get(assistant);
    if (
      previous &&
      (previous.message !== context.message ||
        previous.timestamp !== context.timestamp ||
        !assistant.querySelector("[data-npa-recap]"))
    ) {
      previous.cleanup();
      recaps.delete(assistant);
    }
    if (!recaps.has(assistant)) {
      const cleanup = decorateRecap(assistant);
      if (cleanup)
        recaps.set(assistant, { message: context.message, timestamp: context.timestamp, cleanup });
    }
  }
  function promptBlocks(assistant: Node) {
    const blocks = Array.from(assistant.querySelectorAll('[data-paseo-markdown-tag="pre"]')).filter(
      (b) => !b.closest('[data-paseo-markdown-tag="blockquote"]'),
    );
    const codes = blocks.map((b) =>
      b
        .querySelector('[data-paseo-markdown-tag="code"]')
        ?.textContent?.replace(/\r\n/g, "\n")
        .replace(/\n+$/, ""),
    );
    return { blocks, codes };
  }
  function markHeadings(assistant: Node, marked?: Set<Node>) {
    for (const heading of messageParts(assistant).flatMap((part) =>
      Array.from(
        part.querySelectorAll('[data-paseo-markdown-tag="h2"], [data-paseo-markdown-tag="h3"]'),
      ),
    )) {
      if (
        !/^what(?:['’]s)? next$|^next steps$/i.test(heading.textContent?.trim() ?? "") ||
        heading.closest('[data-paseo-markdown-tag="blockquote"]')
      )
        continue;
      marked?.add(heading);
      if (!headings.has(heading)) {
        headings.set(heading, heading.getAttribute("data-npa-next-heading"));
        heading.setAttribute("data-npa-next-heading", "true");
      }
    }
  }
  // Virtualized history mounts rows as the user scrolls. Waiting for the timeline read let the raw
  // Markdown paint first, then the panel replaced it and the list re-measured the rows, which
  // made scrolling jump. Mutation callbacks run before paint, so earlier replies fold here
  // synchronously; the scan still upgrades the current reply to its interactive panel.
  function paintNow(assistants: Node[]) {
    const parts = [...new Set(assistants.flatMap(messageParts))];
    for (const part of parts) {
      const context = identify(part);
      if (context) decorate(part, context);
    }
    for (const part of parts) {
      const context = identify(part);
      if (!context) continue;
      const { blocks, codes } = promptBlocks(part);
      blocks.forEach((block, index) => {
        const current = owned.get(block);
        if (current) {
          if (!current.intact()) current.rerender();
          return;
        }
        const shown = readOnly(block, part, context, codes[index] ?? "");
        if (!shown.length) return;
        render(block, context, shown, idle, true);
        markHeadings(part);
      });
    }
  }
  async function scan() {
    if (stopped) return;
    if (running) {
      rescan = true;
      return;
    }
    running = true;
    const refresh = fresh;
    fresh = false;
    const found = new Set<Node>();
    const reads = new Set<string>();
    const recapMessages = new Set<Node>();
    const nextHeadings = new Set<Node>();
    const queried = Array.from(doc.querySelectorAll('[data-testid="assistant-message"]'));
    try {
      for (const assistant of queried) {
        const context = identify(assistant);
        if (!context) continue;
        recapMessages.add(assistant);
        decorate(assistant, context);
        const { blocks, codes } = promptBlocks(assistant);
        if (
          !codes.some(
            (code) =>
              /^prompt:/i.test(code ?? "") ||
              (code?.trimStart().startsWith("{") && parseNextPrompts(code)),
          )
        )
          continue;
        const id = JSON.stringify([context.serverId, context.agentId, context.workspaceId]);
        const key = JSON.stringify([context.message, context.timestamp, codes]);
        let entry = cache.get(id);
        if (!reads.has(id) && (refresh || !entry?.seen.has(key))) {
          entry = { seen: new Set(), snapshot: controller.inspect(context) };
          cache.set(id, entry);
          reads.add(id);
        }
        if (!entry) continue;
        entry.seen.add(key);
        let snapshot: Snapshot;
        try {
          snapshot = await entry.snapshot;
        } catch {
          // A failed read must not strip panels that already render read-only.
          cache.delete(id);
          snapshot = idle;
        }
        if (stopped) break;
        for (const [index, block] of blocks.entries()) {
          const code = codes[index];
          const candidates = snapshot.candidates.filter(
            (c) =>
              c.block === code &&
              answers(c, context.timestamp) &&
              c.source.includes(context.message),
          );
          if (candidates.length) {
            if (!valid(block, context, candidates[0])) continue;
            // Identical structured fences cannot share a selection across separate declarations.
            if (
              new Set(candidates.flatMap((c) => (c.selection ? [c.selection.blockKey] : []))).size >
              1
            )
              continue;
            found.add(block);
            render(block, context, candidates, snapshot);
          } else {
            const shown = readOnly(block, assistant, context, code ?? "");
            if (!shown.length) continue;
            found.add(block);
            render(block, context, shown, snapshot, true);
          }
          markHeadings(assistant, nextHeadings);
        }
      }
      // Rows mounted while this scan awaited the read were folded by paintNow; this scan never saw
      // them, so it only settles nodes it queried or nodes that left the page.
      const seen = new Set(queried);
      const settled = (node: Node) =>
        !node.isConnected || seen.has(node.closest('[data-testid="assistant-message"]') ?? node);
      for (const block of owned.keys()) if (!found.has(block) && settled(block)) clear(block);
      for (const [message, recap] of recaps)
        if (!recapMessages.has(message) && settled(message)) {
          recap.cleanup();
          recaps.delete(message);
        }
      for (const [heading, prior] of headings)
        if (!nextHeadings.has(heading) && settled(heading)) {
          if (prior === null) heading.removeAttribute("data-npa-next-heading");
          else heading.setAttribute("data-npa-next-heading", prior);
          headings.delete(heading);
        }
    } finally {
      running = false;
      if (rescan) {
        rescan = false;
        schedule();
      }
    }
  }
  function schedule() {
    if (scheduled || stopped) return;
    scheduled = true;
    scanTimer = setTimeout(() => {
      scheduled = false;
      void scan();
    }, 100);
  }
  // Our own note and button updates must not trigger another timeline read.
  function ours(node: Node) {
    const element = typeof node.closest === "function" ? node : node.parentElement;
    return !!element?.closest(`[${OWNER}]`);
  }
  const observer = new MutationObserver((records) => {
    if (stopped) return;
    const mounted: Node[] = [];
    for (const record of records)
      for (const node of Array.from(record.addedNodes)) {
        if (ours(node)) continue;
        if (node.getAttribute?.("data-testid") === "assistant-message") mounted.push(node);
        else
          mounted.push(
            ...Array.from(node.querySelectorAll?.('[data-testid="assistant-message"]') ?? []),
          );
      }
    if (mounted.length) paintNow(mounted);
    const foreign = records.some((record) => {
      if (ours(record.target)) return false;
      const nodes = [...Array.from(record.addedNodes), ...Array.from(record.removedNodes)];
      return !nodes.length || !nodes.every(ours);
    });
    if (foreign) schedule();
  });
  observer.observe(doc.body, { childList: true, subtree: true, characterData: true });
  const timer = setInterval(() => {
    fresh = true;
    schedule();
  }, 2500);
  schedule();
  return () => {
    stopped = true;
    observer.disconnect();
    doc.removeEventListener?.("keydown", onKey);
    doc.removeEventListener?.("keyup", onKey);
    clearInterval(timer);
    clearTimeout(scanTimer);
    for (const block of owned.keys()) clear(block);
    for (const recap of recaps.values()) recap.cleanup();
    for (const [heading, prior] of headings) {
      if (prior === null) heading.removeAttribute("data-npa-next-heading");
      else heading.setAttribute("data-npa-next-heading", prior);
    }
    style.remove();
  };
}
