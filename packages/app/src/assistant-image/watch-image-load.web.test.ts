import { describe, expect, it, vi } from "vitest";
import { type WatchedImage, watchImageLoadDimensions } from "./watch-image-load.web";

class FakeImage implements WatchedImage {
  src = "";
  naturalWidth = 0;
  naturalHeight = 0;
  private readonly listeners = new Set<() => void>();

  addEventListener(_type: "load", listener: () => void): void {
    this.listeners.add(listener);
  }

  removeEventListener(_type: "load", listener: () => void): void {
    this.listeners.delete(listener);
  }

  finishLoading(width: number, height: number): void {
    this.naturalWidth = width;
    this.naturalHeight = height;
    for (const listener of this.listeners) {
      listener();
    }
  }
}

describe("watchImageLoadDimensions", () => {
  it("reports natural dimensions on load without waiting for decode", () => {
    const image = new FakeImage();
    const onLoad = vi.fn();

    watchImageLoadDimensions("blob:shot", onLoad, () => image);
    image.finishLoading(500, 600);

    expect(image.src).toBe("blob:shot");
    expect(onLoad).toHaveBeenCalledWith({ width: 500, height: 600 });
  });

  it("ignores a load that finishes after the watch stops", () => {
    const image = new FakeImage();
    const onLoad = vi.fn();

    const stop = watchImageLoadDimensions("blob:shot", onLoad, () => image);
    stop();
    image.finishLoading(500, 600);

    expect(onLoad).not.toHaveBeenCalled();
  });

  it("ignores a load without usable dimensions", () => {
    const image = new FakeImage();
    const onLoad = vi.fn();

    watchImageLoadDimensions("blob:shot", onLoad, () => image);
    image.finishLoading(0, 0);

    expect(onLoad).not.toHaveBeenCalled();
  });
});
