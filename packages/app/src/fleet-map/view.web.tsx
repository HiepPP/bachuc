import {
  type CSSProperties,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
  type PointerEvent,
  type RefObject,
} from "react";
import { Text, View } from "react-native";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import { useTranslation } from "react-i18next";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import type { Theme } from "@/styles/theme";
import { navigateToAgent } from "@/utils/navigate-to-agent";
import type { SidebarStateBucket } from "@/utils/sidebar-agent-state";
import { getStatusDotColor } from "@/utils/status-dot-color";
import { computeFleetLayout, type FleetLayout } from "./layout";
import type { FleetNode } from "./nodes";
import { createRenderScheduler, type RenderScheduler } from "./render-scheduler";
import type { FleetScene, FleetSceneGraph } from "./scene";
import { useFleetNodes } from "./use-fleet-nodes";

export interface FleetMapViewProps {
  serverId: string;
}

export interface FleetPalette {
  buckets: Record<SidebarStateBucket, number>;
  edge: number;
}

// Everything the view needs from the lazy chunk. `dispose` releases the shared WebGL renderer.
export interface FleetMapRuntime {
  scene: FleetScene;
  maxPixelRatio: number;
  draw: (canvas: HTMLCanvasElement) => void;
  dispose: () => void;
}

export interface FleetMapFrameSource {
  requestFrame: (callback: () => void) => number;
  cancelFrame: (handle: number) => void;
  now: () => number;
}

export type FleetMapStatus = "loading" | "ready" | "unavailable";

const PULSE_PERIOD_MS = 1600;
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";
const FALLBACK_COLOR = 0x888888;

const browserFrames: FleetMapFrameSource = {
  requestFrame: (callback) => window.requestAnimationFrame(callback),
  cancelFrame: (handle) => window.cancelAnimationFrame(handle),
  now: () => performance.now(),
};

export function parseHexColor(value: string | null | undefined): number | null {
  const match = /^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(value ?? "");
  if (!match) {
    return null;
  }
  let digits = match[1];
  if (digits.length === 3) {
    digits = [...digits].map((digit) => digit + digit).join("");
  }
  return Number.parseInt(digits.slice(0, 6), 16);
}

export function buildFleetPalette(theme: Theme): FleetPalette {
  const fallback = parseHexColor(theme.colors.border) ?? FALLBACK_COLOR;
  const colorOf = (bucket: SidebarStateBucket) =>
    parseHexColor(getStatusDotColor({ theme, bucket, showDoneAsInactive: true })) ?? fallback;
  return {
    buckets: {
      needs_input: colorOf("needs_input"),
      failed: colorOf("failed"),
      running: colorOf("running"),
      attention: colorOf("attention"),
      done: colorOf("done"),
    },
    edge: parseHexColor(theme.colors.foregroundMuted) ?? fallback,
  };
}

// withUnistyles calls the mapping on every render. One palette per theme keeps the reference
// stable, so an unchanged theme never invalidates the scene.
const paletteByTheme = new WeakMap<Theme, FleetPalette>();

function getFleetPalette(theme: Theme): FleetPalette {
  const cached = paletteByTheme.get(theme);
  if (cached) {
    return cached;
  }
  const palette = buildFleetPalette(theme);
  paletteByTheme.set(theme, palette);
  return palette;
}

const paletteMapping = (theme: Theme) => ({ palette: getFleetPalette(theme) });
const foregroundMutedMapping = (theme: Theme) => ({ color: theme.colors.foregroundMuted });

function buildSceneGraph(
  nodes: readonly FleetNode[],
  layout: FleetLayout,
  palette: FleetPalette,
): FleetSceneGraph {
  const sceneNodes: FleetSceneGraph["nodes"][number][] = [];
  for (const node of nodes) {
    const position = layout.positions.get(node.id);
    if (position) {
      sceneNodes.push({
        id: node.id,
        position,
        color: palette.buckets[node.bucket],
        pulsing: node.bucket === "running",
      });
    }
  }
  return {
    nodes: sceneNodes,
    edges: layout.edges,
    boundingRadius: layout.boundingRadius,
    edgeColor: palette.edge,
  };
}

// Loads three.js and the scene in their own chunk. Rejects when WebGL is not available.
export async function loadFleetMapRuntime(): Promise<FleetMapRuntime> {
  const [THREE, rendererModule, sceneModule] = await Promise.all([
    import("three"),
    import("./shared-renderer"),
    import("./scene"),
  ]);
  const handle = rendererModule.acquireSharedRenderer();
  try {
    const scene = sceneModule.createFleetScene({ THREE });
    return {
      scene,
      maxPixelRatio: sceneModule.MAX_PIXEL_RATIO,
      // Blit in the same task as the render: the drawing buffer is gone after compositing.
      draw: (canvas) => {
        scene.renderWith(handle.renderer);
        rendererModule.blitToCanvas(handle.renderer, canvas);
      },
      dispose: () => {
        scene.dispose();
        handle.release();
      },
    };
  } catch (error) {
    handle.release();
    throw error;
  }
}

interface FleetMapSession {
  runtime: FleetMapRuntime;
  scheduler: RenderScheduler;
}

interface HoverState {
  id: string;
  x: number;
  y: number;
}

export interface UseFleetMapDriverOptions {
  nodes: readonly FleetNode[];
  palette: FleetPalette;
  onOpenNode: (id: string) => void;
  // Read once at mount.
  loadRuntime?: () => Promise<FleetMapRuntime>;
  frames?: FleetMapFrameSource;
}

export interface FleetMapDriver {
  canvasRef: RefObject<HTMLCanvasElement | null>;
  status: FleetMapStatus;
  hovered: { node: FleetNode; x: number; y: number } | null;
  onPointerMove: (event: PointerEvent<HTMLCanvasElement>) => void;
  onPointerLeave: () => void;
  onClick: (event: MouseEvent<HTMLCanvasElement>) => void;
}

// Wires the scheduler to the window and the canvas. `teardown` also disposes the runtime.
function attachRuntime(
  runtime: FleetMapRuntime,
  canvas: HTMLCanvasElement,
  frames: FleetMapFrameSource,
): { scheduler: RenderScheduler; teardown: () => void } {
  const scheduler = createRenderScheduler({
    ...frames,
    render: (time) => {
      runtime.scene.setPulsePhase((time % PULSE_PERIOD_MS) / PULSE_PERIOD_MS);
      runtime.draw(canvas);
    },
  });

  const syncVisible = () => scheduler.setVisible(document.visibilityState === "visible");
  const onFocus = () => scheduler.setFocused(true);
  const onBlur = () => scheduler.setFocused(false);
  const motion = window.matchMedia(REDUCED_MOTION_QUERY);
  const syncMotion = () => scheduler.setReducedMotion(motion.matches);
  syncVisible();
  scheduler.setFocused(document.hasFocus());
  syncMotion();
  document.addEventListener("visibilitychange", syncVisible);
  window.addEventListener("focus", onFocus);
  window.addEventListener("blur", onBlur);
  motion.addEventListener("change", syncMotion);

  const observer = new ResizeObserver((entries) => {
    const entry = entries[entries.length - 1];
    if (!entry) {
      return;
    }
    const { width, height } = entry.contentRect;
    const ratio = Math.min(window.devicePixelRatio || 1, runtime.maxPixelRatio);
    canvas.width = Math.max(1, Math.round(width * ratio));
    canvas.height = Math.max(1, Math.round(height * ratio));
    runtime.scene.resize(width, height, ratio);
    scheduler.invalidate();
  });
  observer.observe(canvas);

  return {
    scheduler,
    teardown: () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", syncVisible);
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
      motion.removeEventListener("change", syncMotion);
      scheduler.dispose();
      runtime.dispose();
    },
  };
}

function pointInCanvas(event: { clientX: number; clientY: number; currentTarget: Element }) {
  const rect = event.currentTarget.getBoundingClientRect();
  return { x: event.clientX - rect.left, y: event.clientY - rect.top };
}

export function useFleetMapDriver(options: UseFleetMapDriverOptions): FleetMapDriver {
  const { nodes, palette, onOpenNode } = options;
  const [fixed] = useState(() => ({
    loadRuntime: options.loadRuntime ?? loadFleetMapRuntime,
    frames: options.frames ?? browserFrames,
  }));
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const hoveredIdRef = useRef<string | null>(null);
  const [session, setSession] = useState<FleetMapSession | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  const [hover, setHover] = useState<HoverState | null>(null);

  useEffect(() => {
    let cancelled = false;
    let teardown: (() => void) | null = null;

    const start = async () => {
      let runtime: FleetMapRuntime;
      try {
        runtime = await fixed.loadRuntime();
      } catch {
        if (!cancelled) {
          setUnavailable(true);
        }
        return;
      }
      const canvas = canvasRef.current;
      if (cancelled || !canvas) {
        runtime.dispose();
        return;
      }
      const attached = attachRuntime(runtime, canvas, fixed.frames);
      teardown = attached.teardown;
      setSession({ runtime, scheduler: attached.scheduler });
    };
    void start();

    return () => {
      cancelled = true;
      teardown?.();
      setSession(null);
    };
  }, [fixed]);

  const layout = useMemo(() => computeFleetLayout(nodes), [nodes]);

  useEffect(() => {
    if (!session) {
      return;
    }
    session.runtime.scene.setGraph(buildSceneGraph(nodes, layout, palette));
    session.scheduler.setPulsing(nodes.some((node) => node.bucket === "running"));
    session.scheduler.invalidate();
  }, [session, nodes, layout, palette]);

  const setHoveredNode = useCallback(
    (next: HoverState | null) => {
      const nextId = next?.id ?? null;
      if (!session || hoveredIdRef.current === nextId) {
        return;
      }
      hoveredIdRef.current = nextId;
      session.runtime.scene.setHovered(nextId);
      session.scheduler.invalidate();
      setHover(next);
    },
    [session],
  );

  const onPointerMove = useCallback(
    (event: PointerEvent<HTMLCanvasElement>) => {
      if (!session) {
        return;
      }
      const { x, y } = pointInCanvas(event);
      const id = session.runtime.scene.pick(x, y);
      setHoveredNode(id === null ? null : { id, x, y });
    },
    [session, setHoveredNode],
  );

  const onPointerLeave = useCallback(() => setHoveredNode(null), [setHoveredNode]);

  const onClick = useCallback(
    (event: MouseEvent<HTMLCanvasElement>) => {
      if (!session) {
        return;
      }
      const { x, y } = pointInCanvas(event);
      const id = session.runtime.scene.pick(x, y);
      if (id !== null) {
        onOpenNode(id);
      }
    },
    [session, onOpenNode],
  );

  const hoveredNode = hover ? nodes.find((node) => node.id === hover.id) : undefined;
  let status: FleetMapStatus = "loading";
  if (unavailable) {
    status = "unavailable";
  } else if (session) {
    status = "ready";
  }

  return {
    canvasRef,
    status,
    hovered: hover && hoveredNode ? { node: hoveredNode, x: hover.x, y: hover.y } : null,
    onPointerMove,
    onPointerLeave,
    onClick,
  };
}

const ThemedLoadingSpinner = withUnistyles(LoadingSpinner);

const canvasStyle: CSSProperties = {
  position: "absolute",
  top: 0,
  left: 0,
  width: "100%",
  height: "100%",
};
const canvasHoveredStyle: CSSProperties = { ...canvasStyle, cursor: "pointer" };

interface FleetMapCanvasProps {
  serverId: string;
  nodes: readonly FleetNode[];
  palette: FleetPalette;
}

function FleetMapCanvas({ serverId, nodes, palette }: FleetMapCanvasProps) {
  const { t } = useTranslation();
  const onOpenNode = useCallback(
    (agentId: string) => {
      navigateToAgent({ serverId, agentId });
    },
    [serverId],
  );
  const driver = useFleetMapDriver({ nodes, palette, onOpenNode });
  const { hovered, status } = driver;

  return (
    <View style={styles.container}>
      <canvas
        ref={driver.canvasRef}
        style={hovered ? canvasHoveredStyle : canvasStyle}
        onPointerMove={driver.onPointerMove}
        onPointerLeave={driver.onPointerLeave}
        onClick={driver.onClick}
      />
      {status === "loading" ? (
        <View style={styles.overlay} pointerEvents="none">
          <ThemedLoadingSpinner size="large" uniProps={foregroundMutedMapping} />
        </View>
      ) : null}
      {status === "unavailable" ? (
        <View style={styles.overlay} pointerEvents="none">
          <Text style={styles.message}>{t("panels.fleetMap.webglUnavailable")}</Text>
        </View>
      ) : null}
      {hovered ? (
        <View
          style={[styles.label, { left: hovered.x + LABEL_OFFSET, top: hovered.y + LABEL_OFFSET }]}
          pointerEvents="none"
        >
          <Text style={styles.labelTitle} numberOfLines={1}>
            {hovered.node.title ?? hovered.node.id}
          </Text>
          {hovered.node.workspaceName ? (
            <Text style={styles.labelSubtitle} numberOfLines={1}>
              {hovered.node.workspaceName}
            </Text>
          ) : null}
        </View>
      ) : null}
    </View>
  );
}

const ThemedFleetMapCanvas = withUnistyles(FleetMapCanvas);

const LABEL_OFFSET = 14;

export function FleetMapView({ serverId }: FleetMapViewProps) {
  const { t } = useTranslation();
  const nodes = useFleetNodes(serverId);
  if (nodes.length === 0) {
    return (
      <View style={styles.overlay}>
        <Text style={styles.message}>{t("panels.fleetMap.empty")}</Text>
      </View>
    );
  }
  return <ThemedFleetMapCanvas serverId={serverId} nodes={nodes} uniProps={paletteMapping} />;
}

const styles = StyleSheet.create((theme) => ({
  container: {
    flex: 1,
    position: "relative",
    overflow: "hidden",
  },
  overlay: {
    position: "absolute",
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
    alignItems: "center",
    justifyContent: "center",
    padding: theme.spacing[4],
  },
  message: {
    color: theme.colors.foregroundMuted,
    textAlign: "center",
  },
  label: {
    position: "absolute",
    maxWidth: 260,
    paddingVertical: theme.spacing[1],
    paddingHorizontal: theme.spacing[2],
    borderRadius: theme.borderRadius.md,
    borderWidth: theme.borderWidth[1],
    borderColor: theme.colors.border,
    backgroundColor: theme.colors.surface1,
  },
  labelTitle: {
    color: theme.colors.foreground,
    fontSize: theme.fontSize.sm,
  },
  labelSubtitle: {
    color: theme.colors.foregroundMuted,
    fontSize: theme.fontSize.sm,
  },
}));
