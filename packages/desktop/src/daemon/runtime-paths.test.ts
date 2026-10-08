import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resolveNodeExecPath } from "./runtime-paths";

const mocks = vi.hoisted(() => ({
  existsSync: vi.fn(),
  readdirSync: vi.fn(),
  app: {
    isPackaged: true,
    name: "Paseo",
  },
}));

vi.mock("node:fs", () => ({
  existsSync: mocks.existsSync,
  readdirSync: mocks.readdirSync,
  readFileSync: vi.fn(),
}));

vi.mock("electron", () => ({
  app: mocks.app,
}));

vi.mock("electron-log/main", () => ({
  default: { warn: vi.fn() },
}));

const originalPlatform = process.platform;
const originalExecPath = process.execPath;
const originalResourcesPath = process.resourcesPath;

function setProcessRuntime(input: {
  platform: NodeJS.Platform;
  execPath: string;
  resourcesPath?: string;
}): void {
  Object.defineProperty(process, "platform", {
    configurable: true,
    value: input.platform,
  });
  Object.defineProperty(process, "execPath", {
    configurable: true,
    value: input.execPath,
  });
  Object.defineProperty(process, "resourcesPath", {
    configurable: true,
    value: input.resourcesPath,
  });
}

describe("runtime-paths", () => {
  beforeEach(() => {
    mocks.app.isPackaged = true;
    mocks.app.name = "Paseo";
    mocks.existsSync.mockReturnValue(true);
    mocks.readdirSync.mockReturnValue([]);
    setProcessRuntime({
      platform: "darwin",
      execPath: "/Applications/Paseo.app/Contents/MacOS/Paseo",
      resourcesPath: "/Applications/Paseo.app/Contents/Resources",
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    setProcessRuntime({
      platform: originalPlatform,
      execPath: originalExecPath,
      resourcesPath: originalResourcesPath,
    });
  });

  it("uses the macOS Helper executable for packaged daemon node launches", () => {
    expect(resolveNodeExecPath()).toBe(
      "/Applications/Paseo.app/Contents/Frameworks/Paseo Helper.app/Contents/MacOS/Paseo Helper",
    );
  });

  it("finds the bundled Helper when neither the executable nor the app name matches it", () => {
    const helperPath =
      "/Applications/Paseo.app/Contents/Frameworks/Bachuc Helper.app/Contents/MacOS/Bachuc Helper";
    mocks.app.name = "Bachuc Dev";
    mocks.readdirSync.mockReturnValue(["Bachuc Helper (GPU).app", "Bachuc Helper.app"]);
    mocks.existsSync.mockImplementation(
      (candidate: string) => candidate.endsWith("/Frameworks") || candidate === helperPath,
    );

    expect(resolveNodeExecPath()).toBe(helperPath);
  });
});
