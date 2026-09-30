// Copied from prompt-translate/client/dom.ts. Structural DOM types: the client tsconfig has
// no DOM lib, and linkedom satisfies these in tests.
export interface El {
  textContent: string | null;
  isConnected: boolean;
  parentElement: El | null;
  nextElementSibling: El | null;
  childElementCount: number;
  value?: string;
  style: { cssText: string };
  querySelector(selector: string): El | null;
  querySelectorAll(selector: string): ArrayLike<El>;
  closest(selector: string): El | null;
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
  hasAttribute(name: string): boolean;
  contains(node: El | null): boolean;
  append(...nodes: (El | string)[]): void;
  before(...nodes: El[]): void;
  remove(): void;
  addEventListener(name: string, handler: (event: DomEvent) => void, capture?: boolean): void;
  removeEventListener(name: string, handler: (event: DomEvent) => void, capture?: boolean): void;
  focus?(): void;
  showPopover?(): void;
  getBoundingClientRect?(): { left: number; top: number; bottom: number };
}
export interface DomEvent {
  key?: string;
  newState?: string;
  target: unknown;
  preventDefault(): void;
  stopPropagation(): void;
}
export interface Doc {
  head: El;
  body: El;
  createElement(tag: string): El;
  querySelectorAll(selector: string): ArrayLike<El>;
  defaultView: {
    innerWidth?: number;
    innerHeight?: number;
    getComputedStyle?(node: El): {
      color: string;
      backgroundColor: string;
      fontFamily?: string;
      fontSize?: string;
      fontWeight?: string;
      fontStyle?: string;
      lineHeight?: string;
      letterSpacing?: string;
    };
  } | null;
  addEventListener(name: "click", handler: (event: DomEvent) => void, capture: boolean): void;
  removeEventListener(name: "click", handler: (event: DomEvent) => void, capture: boolean): void;
}
export type Observer = new (callback: () => void) => {
  observe(node: El, options: object): void;
  disconnect(): void;
};

type Fiber = {
  memoizedProps?: Record<string, unknown>;
  return?: Fiber;
  alternate?: Fiber;
  stateNode?: { current?: Fiber };
};

// Private host detail: React stores the fiber on the DOM node under a randomized key.
export function reactProps(
  node: El,
  match: (props: Record<string, unknown>) => boolean,
): Record<string, unknown> | null {
  const key = Object.keys(node).find((name) => name.startsWith("__reactFiber$"));
  if (!key) return null;
  let fiber = (node as unknown as Record<string, Fiber | undefined>)[key];
  // A reused composer can still point at React's previous host/agent buffer.
  let root = fiber;
  for (let depth = 0; root?.return && depth < 200; depth++) root = root.return;
  if (root?.stateNode?.current && root.stateNode.current !== root) {
    fiber = fiber?.alternate;
    if (!fiber) return null;
  }
  for (let depth = 0; fiber && depth < 40; depth++, fiber = fiber.return)
    if (fiber.memoizedProps && match(fiber.memoizedProps)) return fiber.memoizedProps;
  return null;
}

export function desktopSupported(): boolean {
  const scope = globalThis as { document?: unknown; navigator?: { userAgent?: string } };
  return scope.document !== undefined && /Electron\//.test(scope.navigator?.userAgent ?? "");
}

const agentPattern = /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/i;
export const ROOT = '[data-testid="message-input-root"]';
const FIELD = "[data-composer-input], textarea";

export function composerHost(node: El): string | null {
  const field = node.querySelector(FIELD) ?? node;
  const props = reactProps(field, (value) => typeof value.voiceServerId === "string");
  return (props?.voiceServerId as string | undefined) ?? null;
}
// New-thread composers expose a draft key, not an agent UUID, until the agent exists.
export function composerAgent(node: El): string | null {
  const field = node.querySelector(FIELD) ?? node;
  const props = reactProps(field, (value) => typeof value.voiceAgentId === "string");
  const id = props?.voiceAgentId;
  return typeof id === "string" && agentPattern.test(id) ? id : null;
}
