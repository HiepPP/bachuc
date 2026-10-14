import { describe, expect, it, vi } from "vitest";
import { createSharedRendererPool } from "./shared-renderer";

function createFakeRenderer() {
  return { dispose: vi.fn(), forceContextLoss: vi.fn() };
}

describe("createSharedRendererPool", () => {
  it("creates one renderer for two acquires and disposes it on the last release", () => {
    const factory = vi.fn(createFakeRenderer);
    const pool = createSharedRendererPool(factory);

    const first = pool.acquire();
    const second = pool.acquire();
    expect(factory).toHaveBeenCalledTimes(1);
    expect(second.renderer).toBe(first.renderer);

    first.release();
    expect(first.renderer.dispose).not.toHaveBeenCalled();
    expect(first.renderer.forceContextLoss).not.toHaveBeenCalled();

    second.release();
    expect(first.renderer.dispose).toHaveBeenCalledTimes(1);
    expect(first.renderer.forceContextLoss).toHaveBeenCalledTimes(1);
  });

  it("ignores a second release from the same handle", () => {
    const pool = createSharedRendererPool(vi.fn(createFakeRenderer));
    const first = pool.acquire();
    const second = pool.acquire();

    first.release();
    first.release();
    expect(first.renderer.dispose).not.toHaveBeenCalled();

    second.release();
    expect(first.renderer.dispose).toHaveBeenCalledTimes(1);
  });

  it("creates a fresh renderer after the last release", () => {
    const factory = vi.fn(createFakeRenderer);
    const pool = createSharedRendererPool(factory);

    const first = pool.acquire();
    first.release();
    const next = pool.acquire();

    expect(factory).toHaveBeenCalledTimes(2);
    expect(next.renderer).not.toBe(first.renderer);
  });

  it("leaves the pool empty when the factory throws", () => {
    const factory = vi.fn(() => {
      throw new Error("Error creating WebGL context.");
    });
    const pool = createSharedRendererPool(factory);

    expect(() => pool.acquire()).toThrow("Error creating WebGL context.");
    expect(() => pool.acquire()).toThrow("Error creating WebGL context.");
    expect(factory).toHaveBeenCalledTimes(2);
  });
});
