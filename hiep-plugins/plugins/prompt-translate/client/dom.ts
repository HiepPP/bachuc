// Structural DOM types: the client tsconfig has no DOM lib, and linkedom satisfies these in tests.
export interface El {
  textContent: string | null;
  isConnected: boolean;
  parentElement: El | null;
  nextElementSibling: El | null;
  childElementCount: number;
  value?: string;
  selected?: boolean;
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
  after(...nodes: El[]): void;
  before(...nodes: El[]): void;
  remove(): void;
  addEventListener(name: string, handler: (event: DomEvent) => void, capture?: boolean): void;
  removeEventListener(name: string, handler: (event: DomEvent) => void, capture?: boolean): void;
  focus?(): void;
  showPopover?(): void;
  getBoundingClientRect?(): { left: number; top: number; bottom: number };
  setSelectionRange?(start: number, end: number): void;
  dispatchEvent(event: object): boolean;
  click(): void;
}
export interface DomEvent {
  key?: string;
  newState?: string;
  target: unknown;
  preventDefault(): void;
  stopPropagation(): void;
  stopImmediatePropagation(): void;
}
export type Key = {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  isComposing?: boolean;
  keyCode?: number;
  target: unknown;
  preventDefault(): void;
  stopImmediatePropagation(): void;
};
export interface Doc {
  head: El;
  body: El;
  createElement(tag: string): El;
  querySelectorAll(selector: string): ArrayLike<El>;
  defaultView: {
    innerWidth?: number;
    innerHeight?: number;
    Event: new (type: string, init?: { bubbles?: boolean }) => object;
    KeyboardEvent?: new (
      type: string,
      init: {
        key: string;
        code?: string;
        bubbles?: boolean;
        cancelable?: boolean;
        metaKey?: boolean;
        ctrlKey?: boolean;
      },
    ) => object;
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
  addEventListener(name: "keydown", handler: (event: Key) => void, capture: boolean): void;
  addEventListener(name: "click", handler: (event: DomEvent) => void, capture: boolean): void;
  removeEventListener(name: "keydown", handler: (event: Key) => void, capture: boolean): void;
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
