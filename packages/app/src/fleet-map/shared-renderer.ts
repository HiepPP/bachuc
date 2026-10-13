import { WebGLRenderer } from "three";

export interface DisposableRenderer {
  dispose: () => void;
  forceContextLoss: () => void;
}

export interface SharedRendererHandle<R> {
  renderer: R;
  release: () => void;
}

export interface SharedRendererPool<R> {
  acquire: () => SharedRendererHandle<R>;
}

// One WebGL context for the whole app (Render Budget). Each view renders into the shared
// renderer and copies the frame to its own 2D canvas with `blitToCanvas`.
export function createSharedRendererPool<R extends DisposableRenderer>(
  createRenderer: () => R,
): SharedRendererPool<R> {
  let renderer: R | null = null;
  let users = 0;

  return {
    acquire: () => {
      const current = renderer ?? createRenderer();
      renderer = current;
      users += 1;
      let released = false;
      return {
        renderer: current,
        release: () => {
          if (released) {
            return;
          }
          released = true;
          users -= 1;
          if (users === 0 && renderer === current) {
            renderer = null;
            current.dispose();
            current.forceContextLoss();
          }
        },
      };
    },
  };
}

function createWebGLRenderer(): WebGLRenderer {
  const renderer = new WebGLRenderer({
    antialias: true,
    alpha: true,
    powerPreference: "low-power",
  });
  renderer.setClearColor(0x000000, 0);
  return renderer;
}

const sharedPool = createSharedRendererPool(createWebGLRenderer);

// Throws when the browser cannot create a WebGL context.
export function acquireSharedRenderer(): SharedRendererHandle<WebGLRenderer> {
  return sharedPool.acquire();
}

// Call right after `render`: the WebGL drawing buffer is only valid until the browser composites.
export function blitToCanvas(renderer: WebGLRenderer, target: HTMLCanvasElement): void {
  const context = target.getContext("2d");
  if (!context) {
    return;
  }
  context.clearRect(0, 0, target.width, target.height);
  context.drawImage(renderer.domElement, 0, 0, target.width, target.height);
}
