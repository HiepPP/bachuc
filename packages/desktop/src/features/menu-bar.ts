import path from "node:path";
import { Menu, Tray, nativeImage } from "electron";
import log from "electron-log/main";

// The release app runs as a menu bar app with no window (ADR-0074). GPUI is the daily app.

export type MenuBarDaemonState = "starting" | "running" | "stopped" | "errored";

export interface MenuBarStatus {
  status: MenuBarDaemonState;
  listen: string | null;
}

export type MenuBarItemId = "status" | "open-gpui" | "restart" | "logs" | "quit";

export interface MenuBarItem {
  id: MenuBarItemId;
  label: string;
  enabled: boolean;
}

function isUp(status: MenuBarStatus): boolean {
  return status.status === "running" || status.status === "starting";
}

export function needsDaemonStart(status: MenuBarStatus): boolean {
  return !isUp(status);
}

function statusLabel(status: MenuBarStatus): string {
  if (status.status === "running") {
    return `Daemon running · ${status.listen ?? "unknown address"}`;
  }
  if (status.status === "starting") {
    return "Daemon starting";
  }
  return "Daemon stopped";
}

export function buildMenuBarItems(status: MenuBarStatus): MenuBarItem[] {
  return [
    { id: "status", label: statusLabel(status), enabled: false },
    { id: "open-gpui", label: "Open Bachuc GPUI", enabled: true },
    { id: "restart", label: "Restart daemon", enabled: isUp(status) },
    { id: "logs", label: "Open logs", enabled: true },
    { id: "quit", label: "Quit (daemon keeps running)", enabled: true },
  ];
}

export interface MenuBarDeps {
  resourcesPath: string;
  readStatus: () => Promise<MenuBarStatus>;
  openGpui: () => void;
  restartDaemon: () => Promise<unknown>;
  openLogs: () => void;
  quit: () => void;
}

/** Creates the tray. The menu is rebuilt from a fresh status each time it opens. */
export function startMenuBar(deps: MenuBarDeps): Tray {
  const icon = nativeImage.createFromPath(path.join(deps.resourcesPath, "trayTemplate.png"));
  icon.setTemplateImage(true);
  const tray = new Tray(icon);
  tray.setToolTip("Bachuc daemon");

  const actions: Record<MenuBarItemId, () => void> = {
    status: () => undefined,
    "open-gpui": deps.openGpui,
    restart: () => {
      void deps.restartDaemon().catch((error) => log.error("[menu-bar] restart failed", error));
    },
    logs: deps.openLogs,
    quit: deps.quit,
  };

  const showMenu = async (): Promise<void> => {
    let status: MenuBarStatus;
    try {
      status = await deps.readStatus();
    } catch (error) {
      log.error("[menu-bar] status failed", error);
      status = { status: "errored", listen: null };
    }
    const items = buildMenuBarItems(status);
    const template: Electron.MenuItemConstructorOptions[] = [];
    items.forEach((item, index) => {
      // Separators set the status line and Quit apart from the actions.
      if (index === 1 || index === items.length - 1) {
        template.push({ type: "separator" });
      }
      template.push({ label: item.label, enabled: item.enabled, click: actions[item.id] });
    });
    const menu = Menu.buildFromTemplate(template);
    tray.popUpContextMenu(menu);
  };

  tray.on("click", () => void showMenu());
  tray.on("right-click", () => void showMenu());
  return tray;
}
