/**
 * @vitest-environment jsdom
 */
import React from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Theme } from "@/styles/theme";
import type { FleetNode } from "./nodes";
import type { FleetScene } from "./scene";
import {
  buildFleetPalette,
  FleetMapView,
  parseHexColor,
  useFleetMapDriver,
  type FleetMapRuntime,
  type FleetPalette,
} from "./view.web";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const navigateToAgent = vi.hoisted(() => vi.fn());
vi.mock("@/utils/navigate-to-agent", () => ({ navigateToAgent }));

const THEME = {
  colors: {
    statusDotWarning: "#b37824",
    statusDotDanger: "#f12e2f",
    statusDotRunning: "#268ae0",
    statusDotSuccess: "#299f51",
    border: "#e4e4e7",
    foregroundMuted: "#666666",
  },
} as unknown as Theme;

const PALETTE: FleetPalette = {
  buckets: {
    needs_input: 0xb37824,
    failed: 0xf12e2f,
    running: 0x268ae0,
    attention: 0x299f51,
    done: 0xe4e4e7,
  },
  edge: 0x666666,
};

function makeNode(id: string, bucket: FleetNode["bucket"], parentId: string | null = null) {
  return {
    id,
    title: `Title ${id}`,
    workspaceId: "ws-1",
    workspaceName: "Workspace one",
    parentId,
    bucket,
    createdAt: new Date(1_700_000_000_000),
  } satisfies FleetNode;
}

function createFrames() {
  let time = 0;
  let nextHandle = 1;
  const pending = new Map<number, () => void>();
  return {
    requestFrame: (callback: () => void) => {
      const handle = nextHandle;
      nextHandle += 1;
      pending.set(handle, callback);
      return handle;
    },
    cancelFrame: (handle: number) => {
      pending.delete(handle);
    },
    now: () => time,
    // Advances the clock, then runs the callbacks that were pending at the call.
    advance: (ms: number) => {
      time += ms;
      const callbacks = [...pending.values()];
      pending.clear();
      act(() => {
        for (const callback of callbacks) callback();
      });
      return callbacks.length;
    },
    pendingCount: () => pending.size,
  };
}

function createFakeRuntime(pick: (x: number, y: number) => string | null = () => null) {
  const scene = {
    setGraph: vi.fn(),
    setHovered: vi.fn(),
    setPulsePhase: vi.fn(),
    resize: vi.fn(),
    pick: vi.fn(pick),
    renderWith: vi.fn(),
    dispose: vi.fn(),
  } satisfies FleetScene;
  const runtime = {
    scene,
    maxPixelRatio: 1.5,
    draw: vi.fn<FleetMapRuntime["draw"]>(),
    dispose: vi.fn<FleetMapRuntime["dispose"]>(),
  } satisfies FleetMapRuntime;
  return runtime;
}

let resizeCallback:
  | ((entries: Array<{ contentRect: { width: number; height: number } }>) => void)
  | null = null;

beforeEach(() => {
  vi.stubGlobal("React", React);
  resizeCallback = null;
  vi.spyOn(document, "hasFocus").mockReturnValue(true);
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(callback: NonNullable<typeof resizeCallback>) {
        resizeCallback = callback;
      }
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  navigateToAgent.mockReset();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

interface HarnessProps {
  nodes: readonly FleetNode[];
  runtime: Promise<FleetMapRuntime>;
  frames: ReturnType<typeof createFrames>;
  onOpenNode?: (id: string) => void;
}

function Harness({ nodes, runtime, frames, onOpenNode }: HarnessProps) {
  const driver = useFleetMapDriver({
    nodes,
    palette: PALETTE,
    loadRuntime: () => runtime,
    frames,
    onOpenNode: onOpenNode ?? noop,
  });
  return (
    <div>
      <canvas
        data-testid="canvas"
        ref={driver.canvasRef}
        onPointerMove={driver.onPointerMove}
        onPointerLeave={driver.onPointerLeave}
        onClick={driver.onClick}
      />
      {driver.hovered ? (
        <span data-testid="label">
          {driver.hovered.node.title}|{driver.hovered.node.workspaceName}
        </span>
      ) : null}
      <span data-testid="status">{driver.status}</span>
    </div>
  );
}

const noop = () => {};

async function mountReady(props: Omit<HarnessProps, "runtime" | "frames">) {
  const runtime = createFakeRuntime();
  return mountWithRuntime(runtime, props);
}

async function mountWithRuntime(
  runtime: ReturnType<typeof createFakeRuntime>,
  props: Omit<HarnessProps, "runtime" | "frames">,
) {
  const frames = createFrames();
  const view = render(<Harness {...props} runtime={Promise.resolve(runtime)} frames={frames} />);
  await act(async () => {});
  return { runtime, frames, view };
}

// jsdom has no PointerEvent, so fireEvent.pointerMove would drop the coordinates.
function movePointer(target: Element, clientX: number, clientY: number) {
  act(() => {
    target.dispatchEvent(new MouseEvent("pointermove", { clientX, clientY, bubbles: true }));
  });
}

describe("parseHexColor", () => {
  it("reads 3, 6, and 8 digit hex", () => {
    expect(parseHexColor("#268ae0")).toBe(0x268ae0);
    expect(parseHexColor("#fff")).toBe(0xffffff);
    expect(parseHexColor("#299f51ff")).toBe(0x299f51);
  });

  it("returns null for anything else", () => {
    expect(parseHexColor("rgba(0, 0, 0, 0.5)")).toBeNull();
    expect(parseHexColor(null)).toBeNull();
  });
});

describe("buildFleetPalette", () => {
  it("colors each bucket like the sidebar dot and shows done as inactive", () => {
    const palette = buildFleetPalette(THEME);
    expect(palette.buckets).toEqual(PALETTE.buckets);
    expect(palette.edge).toBe(0x666666);
  });
});

describe("FleetMapView empty state", () => {
  it("shows the empty message and no canvas when the host has no agents", () => {
    const { container } = render(<FleetMapView serverId="host-without-session" />);
    expect(screen.getByText("panels.fleetMap.empty")).toBeTruthy();
    expect(container.querySelector("canvas")).toBeNull();
  });
});

describe("useFleetMapDriver", () => {
  it("loads the runtime, then draws the graph once with status colors and no pulse when idle", async () => {
    const nodes = [makeNode("a", "needs_input"), makeNode("b", "done", "a")];
    const { runtime, frames } = await mountReady({ nodes });

    expect(screen.getByTestId("status").textContent).toBe("ready");
    const graph = runtime.scene.setGraph.mock.calls[0][0];
    expect(
      graph.nodes.map((node: { id: string; color: number; pulsing: boolean }) => [
        node.id,
        node.color,
        node.pulsing,
      ]),
    ).toEqual([
      ["a", 0xb37824, false],
      ["b", 0xe4e4e7, false],
    ]);
    expect(graph.edges).toEqual([["a", "b"]]);
    expect(graph.edgeColor).toBe(0x666666);

    expect(frames.advance(100)).toBe(1);
    expect(runtime.draw).toHaveBeenCalledTimes(1);
    expect(runtime.draw).toHaveBeenCalledWith(screen.getByTestId("canvas"));

    // Idle: no further frame is requested.
    expect(frames.pendingCount()).toBe(0);
    expect(frames.advance(1000)).toBe(0);
    expect(runtime.draw).toHaveBeenCalledTimes(1);
  });

  it("pulses only while a node is running, and stops when the window blurs", async () => {
    const { runtime, frames, view } = await mountReady({
      nodes: [makeNode("a", "running"), makeNode("b", "done")],
    });
    const pulsing = runtime.scene.setGraph.mock.calls[0][0].nodes.map(
      (node: { pulsing: boolean }) => node.pulsing,
    );
    expect(pulsing).toEqual([true, false]);

    frames.advance(100);
    frames.advance(100);
    expect(runtime.draw.mock.calls.length).toBeGreaterThanOrEqual(2);

    act(() => {
      window.dispatchEvent(new Event("blur"));
    });
    frames.advance(100);
    const drawsAfterBlur = runtime.draw.mock.calls.length;
    expect(frames.pendingCount()).toBe(0);
    frames.advance(1000);
    expect(runtime.draw.mock.calls.length).toBe(drawsAfterBlur);

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    frames.advance(100);
    expect(runtime.draw.mock.calls.length).toBeGreaterThan(drawsAfterBlur);

    const stopped = [makeNode("a", "done")];
    view.rerender(<Harness nodes={stopped} runtime={Promise.resolve(runtime)} frames={frames} />);
    frames.advance(100);
    const drawsAfterStop = runtime.draw.mock.calls.length;
    expect(frames.advance(1000)).toBe(0);
    expect(runtime.draw.mock.calls.length).toBe(drawsAfterStop);
  });

  it("stops all frames while the tab is hidden", async () => {
    const { runtime, frames } = await mountReady({ nodes: [makeNode("a", "running")] });
    frames.advance(100);
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    const draws = runtime.draw.mock.calls.length;
    expect(frames.pendingCount()).toBe(0);
    expect(frames.advance(1000)).toBe(0);
    expect(runtime.draw.mock.calls.length).toBe(draws);
  });

  it("does not pulse when the user prefers reduced motion", async () => {
    const original = window.matchMedia;
    window.matchMedia = ((media: string) =>
      Object.assign(new EventTarget(), { media, matches: true })) as typeof window.matchMedia;
    try {
      const { runtime, frames } = await mountReady({ nodes: [makeNode("a", "running")] });
      frames.advance(100);
      expect(runtime.draw).toHaveBeenCalledTimes(1);
      expect(frames.advance(1000)).toBe(0);
    } finally {
      window.matchMedia = original;
    }
  });

  it("resizes the scene and the canvas bitmap with the pixel ratio capped", async () => {
    vi.stubGlobal("devicePixelRatio", 3);
    const { runtime } = await mountReady({ nodes: [makeNode("a", "done")] });
    act(() => {
      resizeCallback?.([{ contentRect: { width: 400, height: 300 } }]);
    });
    expect(runtime.scene.resize).toHaveBeenCalledWith(400, 300, 1.5);
    const canvas = screen.getByTestId("canvas") as HTMLCanvasElement;
    expect([canvas.width, canvas.height]).toEqual([600, 450]);
  });

  it("shows the hover label for a picked node and clears it on leave", async () => {
    const runtime = createFakeRuntime((x, y) => (x === 30 && y === 40 ? "a" : null));
    const { frames } = await mountWithRuntime(runtime, { nodes: [makeNode("a", "done")] });
    const canvas = screen.getByTestId("canvas");
    canvas.getBoundingClientRect = () => ({ left: 10, top: 20 }) as DOMRect;

    movePointer(canvas, 40, 60);
    expect(runtime.scene.pick).toHaveBeenCalledWith(30, 40);
    expect(runtime.scene.setHovered).toHaveBeenLastCalledWith("a");
    expect(screen.getByTestId("label").textContent).toBe("Title a|Workspace one");
    expect(frames.pendingCount()).toBe(1);

    movePointer(canvas, 90, 90);
    expect(runtime.scene.setHovered).toHaveBeenLastCalledWith(null);
    expect(screen.queryByTestId("label")).toBeNull();

    movePointer(canvas, 40, 60);
    fireEvent.pointerLeave(canvas);
    expect(runtime.scene.setHovered).toHaveBeenLastCalledWith(null);
    expect(screen.queryByTestId("label")).toBeNull();
  });

  it("opens the clicked node and ignores a click on empty space", async () => {
    const runtime = createFakeRuntime((x) => (x === 30 ? "a" : null));
    const onOpenNode = vi.fn();
    await mountWithRuntime(runtime, { nodes: [makeNode("a", "done")], onOpenNode });
    const canvas = screen.getByTestId("canvas");
    canvas.getBoundingClientRect = () => ({ left: 10, top: 20 }) as DOMRect;

    fireEvent.click(canvas, { clientX: 40, clientY: 60 });
    expect(onOpenNode).toHaveBeenCalledWith("a");

    fireEvent.click(canvas, { clientX: 99, clientY: 60 });
    expect(onOpenNode).toHaveBeenCalledTimes(1);
  });

  it("reports unavailable and does not throw when the runtime cannot load", async () => {
    const frames = createFrames();
    const nodes = [makeNode("a", "done")];
    render(
      <Harness nodes={nodes} runtime={Promise.reject(new Error("no webgl"))} frames={frames} />,
    );
    await act(async () => {});
    expect(screen.getByTestId("status").textContent).toBe("unavailable");
    expect(frames.pendingCount()).toBe(0);
  });

  it("releases the runtime on unmount", async () => {
    const { runtime, view } = await mountReady({ nodes: [makeNode("a", "running")] });
    view.unmount();
    expect(runtime.dispose).toHaveBeenCalledTimes(1);
  });

  it("releases a runtime that resolves after unmount", async () => {
    const runtime = createFakeRuntime();
    let resolve: (value: FleetMapRuntime) => void = () => {};
    const pending = new Promise<FleetMapRuntime>((done) => {
      resolve = done;
    });
    const nodes = [makeNode("a", "done")];
    const view = render(<Harness nodes={nodes} runtime={pending} frames={createFrames()} />);
    view.unmount();
    await act(async () => {
      resolve(runtime);
    });
    expect(runtime.dispose).toHaveBeenCalledTimes(1);
    expect(runtime.scene.setGraph).not.toHaveBeenCalled();
  });
});
