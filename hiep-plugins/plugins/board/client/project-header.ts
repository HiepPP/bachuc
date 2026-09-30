import { settingsRpc } from "@getpaseo/plugin";
import type { PluginClientContext } from "@getpaseo/plugin/client";
import { Platform } from "react-native";
import { projectColors } from "../shared/project-colors";

export interface HeaderElement {
  textContent: string | null;
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
  style: { setProperty(name: string, value: string): void; removeProperty(name: string): void };
}
declare const document: {
  head: { appendChild(node: unknown): void };
  createElement(tag: string): { textContent: string; remove(): void };
  querySelectorAll(selector: string): Iterable<HeaderElement>;
};

const SUBTITLE = '[data-testid="workspace-header-subtitle"]';
// Paseo styles the subtitle with generated classes, so these rules need !important to win.
export const HEADER_CSS =
  `${SUBTITLE}[data-board-project]{font-weight:700!important;padding:1px 8px 1px 4px!important;` +
  "border-radius:6px;border:1px solid color-mix(in srgb,currentColor 30%,transparent);" +
  "background:color-mix(in srgb,currentColor 10%,transparent)}" +
  `${SUBTITLE}[data-board-project]::before{content:attr(data-board-initial);display:inline-block;` +
  "width:1.3em;height:1.3em;line-height:1.3em;margin-right:6px;border-radius:.35em;text-align:center;" +
  "font-size:.8em;vertical-align:.1em;box-sizing:border-box;border:1px solid currentColor}" +
  `${SUBTITLE}[data-board-hue]{background:hsl(var(--board-hue) 60% 72% / .2)!important;` +
  "border-color:hsl(var(--board-hue) 60% 72% / .8)!important}" +
  `${SUBTITLE}[data-board-hue]::before{border:0;color:#fff;background:hsl(var(--board-hue) 42% 58%)}`;

export function projectInitial(name: string) {
  return Array.from(name.trim())[0]?.toUpperCase() ?? "?";
}

/** Board hue per project display name. Names shared by several projects stay uncolored. */
export function headerHues(
  projects: readonly { projectId: string; projectDisplayName: string }[],
  hues: Readonly<Record<string, number>>,
) {
  const counts = new Map<string, number>();
  for (const { projectDisplayName: name } of projects)
    counts.set(name, (counts.get(name) ?? 0) + 1);
  const result = new Map<string, number>();
  for (const { projectId, projectDisplayName: name } of projects) {
    const hue = hues[projectId];
    if (counts.get(name) === 1 && hue !== undefined) result.set(name, hue);
  }
  return result;
}

/**
 * Marks each header subtitle with its project initial and, when known, the Board hue.
 * Paseo reuses the element across workspaces, so a changed name clears the previous hue.
 * Every connected host runs this; the first host that knows the name colors it.
 */
export function syncHeaders(elements: Iterable<HeaderElement>, hues: ReadonlyMap<string, number>) {
  for (const element of elements) {
    const name = element.textContent?.trim() ?? "";
    if (!name) continue;
    if (element.getAttribute("data-board-project") !== name) {
      element.setAttribute("data-board-project", name);
      element.setAttribute("data-board-initial", projectInitial(name));
      element.removeAttribute("data-board-hue");
      element.style.removeProperty("--board-hue");
    }
    const hue = hues.get(name);
    if (hue !== undefined && element.getAttribute("data-board-hue") === null) {
      element.setAttribute("data-board-hue", String(hue));
      element.style.setProperty("--board-hue", String(hue));
    }
  }
}

// Project names and colors change rarely; the DOM pass is cheap and follows navigation.
const DATA_MS = 10_000;
const SYNC_MS = 500;

export function installProjectHeader(client: Pick<PluginClientContext, "paseo" | "rpc">) {
  if (Platform.OS !== "web" || typeof document === "undefined") return () => {};
  const css = document.createElement("style");
  css.textContent = HEADER_CSS;
  document.head.appendChild(css);
  let hues = new Map<string, number>();
  let stopped = false;
  const sync = () => syncHeaders(document.querySelectorAll(SUBTITLE), hues);
  async function load() {
    try {
      const [{ projects }, saved] = await Promise.all([
        client.paseo.projects.list(),
        client.rpc(settingsRpc(projectColors.id).read, {}),
      ]);
      const parsed = saved.status === "ready" ? projectColors.schema.safeParse(saved.values) : null;
      if (stopped) return;
      hues = headerHues(projects, parsed?.success ? parsed.data.hues : {});
      sync();
    } catch {
      // Keep the last known colors; the neutral mark still identifies the project.
    }
  }
  void load();
  sync();
  const dataTimer = setInterval(load, DATA_MS);
  const syncTimer = setInterval(sync, SYNC_MS);
  return () => {
    stopped = true;
    clearInterval(dataTimer);
    clearInterval(syncTimer);
    css.remove();
    for (const element of document.querySelectorAll(SUBTITLE)) {
      for (const name of ["data-board-project", "data-board-initial", "data-board-hue"])
        element.removeAttribute(name);
      element.style.removeProperty("--board-hue");
    }
  };
}
