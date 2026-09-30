import { Platform } from "react-native";

interface Element {
  closest(selector: string): Element | null;
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
  style: { opacity: string; cursor: string };
}
interface Event {
  type: string;
  key?: string;
  target: Element | null;
  preventDefault(): void;
  stopImmediatePropagation(): void;
}
declare const document: {
  querySelector(selector: string): Element | null;
  addEventListener(name: string, listener: (event: Event) => void, capture: boolean): void;
  removeEventListener(name: string, listener: (event: Event) => void, capture: boolean): void;
};
declare const location: { pathname: string };
declare function atob(data: string): string;

const ROW = '[data-testid="plugin-sidebar-watchtower-board-watchtower"]';
const BLOCKED = ["pointerdown", "pointerup", "mousedown", "mouseup", "touchstart", "touchend"];
const DISABLED_TITLE = "Open a thread to use Watchtower";

// Mirrors Paseo's /h/<server>/workspace/<id> route; ids that are not URL-safe use "b64_" + base64url.
function activeWorkspaceId(): string | null {
  const segment = location.pathname.match(/^\/h\/[^/]+\/workspace\/([^/?#]+)\/?$/)?.[1];
  if (!segment) return null;
  const decoded = decodeURIComponent(segment).trim();
  if (!decoded.startsWith("b64_")) return decoded || null;
  try {
    const base64 = decoded.slice(4).replace(/-/g, "+").replace(/_/g, "/");
    const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
    return new TextDecoder().decode(bytes).trim() || null;
  } catch {
    return null;
  }
}

/**
 * On web, the Watchtower sidebar row opens the current thread's Explorer board, like the Command
 * Center item, and is disabled outside a workspace. Native keeps the sidebar surface.
 */
export function installSidebarShortcut(open: (workspaceId: string) => void) {
  if (Platform.OS !== "web" || typeof document === "undefined") return () => {};
  const intercept = (event: Event) => {
    if (!event.target?.closest(ROW)) return;
    if (event.type === "keydown" && event.key !== "Enter" && event.key !== " ") return;
    // Capture on document runs before React's root listener, so the surface route never opens.
    event.preventDefault();
    event.stopImmediatePropagation();
    if (event.type === "click" || event.type === "keydown") {
      const workspaceId = activeWorkspaceId();
      if (workspaceId) open(workspaceId);
    }
  };
  const types = [...BLOCKED, "click", "keydown"];
  for (const type of types) document.addEventListener(type, intercept, true);
  let last: Element | null = null;
  function sync() {
    const row = document.querySelector(ROW);
    if (!row) return;
    const disabled = activeWorkspaceId() === null;
    if (row === last && row.getAttribute("aria-disabled") === String(disabled)) return;
    last = row;
    row.setAttribute("aria-disabled", String(disabled));
    row.style.opacity = disabled ? "0.4" : "";
    row.style.cursor = disabled ? "not-allowed" : "";
    if (disabled) row.setAttribute("title", DISABLED_TITLE);
    else row.removeAttribute("title");
  }
  sync();
  const timer = setInterval(sync, 250);
  return () => {
    clearInterval(timer);
    for (const type of types) document.removeEventListener(type, intercept, true);
    const row = document.querySelector(ROW);
    if (!row) return;
    row.removeAttribute("aria-disabled");
    row.removeAttribute("title");
    row.style.opacity = "";
    row.style.cursor = "";
  };
}
