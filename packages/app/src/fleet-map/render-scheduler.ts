export const MAX_FRAMES_PER_SECOND = 30;

const MIN_FRAME_INTERVAL_MS = 1000 / MAX_FRAMES_PER_SECOND;

export interface RenderSchedulerOptions {
  requestFrame: (callback: () => void) => number;
  cancelFrame: (handle: number) => void;
  now: () => number;
  render: (time: number) => void;
}

export interface RenderScheduler {
  invalidate: () => void;
  setPulsing: (pulsing: boolean) => void;
  setVisible: (visible: boolean) => void;
  setFocused: (focused: boolean) => void;
  setReducedMotion: (reducedMotion: boolean) => void;
  dispose: () => void;
}

// Renders on demand. Idle costs zero frames: a frame is requested only for a pending invalidate
// or a running pulse. Every render, pulse or not, keeps the 30 fps cap from the Render Budget.
// Starts visible and focused with motion allowed; the view corrects these on mount.
export function createRenderScheduler(options: RenderSchedulerOptions): RenderScheduler {
  const { requestFrame, cancelFrame, now, render } = options;
  let frameHandle: number | null = null;
  let dirty = false;
  let disposed = false;
  let lastRenderAt = Number.NEGATIVE_INFINITY;
  let pulsing = false;
  let visible = true;
  let focused = true;
  let reducedMotion = false;

  const isPulseActive = () => pulsing && visible && focused && !reducedMotion;

  const cancel = () => {
    if (frameHandle !== null) {
      cancelFrame(frameHandle);
      frameHandle = null;
    }
  };

  const schedule = () => {
    if (disposed || frameHandle !== null || !visible) {
      return;
    }
    if (dirty || isPulseActive()) {
      frameHandle = requestFrame(onFrame);
    }
  };

  function onFrame() {
    frameHandle = null;
    if (disposed) {
      return;
    }
    const time = now();
    if ((dirty || isPulseActive()) && time - lastRenderAt >= MIN_FRAME_INTERVAL_MS) {
      dirty = false;
      lastRenderAt = time;
      render(time);
    }
    schedule();
  }

  const update = () => {
    if (!visible || (!dirty && !isPulseActive())) {
      cancel();
      return;
    }
    schedule();
  };

  return {
    invalidate: () => {
      dirty = true;
      schedule();
    },
    setPulsing: (next) => {
      pulsing = next;
      update();
    },
    setVisible: (next) => {
      visible = next;
      update();
    },
    setFocused: (next) => {
      focused = next;
      update();
    },
    setReducedMotion: (next) => {
      reducedMotion = next;
      update();
    },
    dispose: () => {
      disposed = true;
      cancel();
    },
  };
}
