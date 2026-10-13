import type { AssistantImageDimensions } from "./load-dimensions";

export interface WatchedImage {
  src: string;
  naturalWidth: number;
  naturalHeight: number;
  addEventListener(type: "load", listener: () => void): void;
  removeEventListener(type: "load", listener: () => void): void;
}

// react-native-web calls Image onLoad only after image.decode() settles, and Chromium
// keeps decode() pending while the window is hidden. The load event alone carries the
// natural size, so a hidden window still gets the right frame.
export function watchImageLoadDimensions(
  uri: string,
  onLoad: (dimensions: AssistantImageDimensions) => void,
  createImage: () => WatchedImage = () => new window.Image(),
): () => void {
  const image = createImage();
  const handleLoad = () => {
    if (image.naturalWidth > 0 && image.naturalHeight > 0) {
      onLoad({ width: image.naturalWidth, height: image.naturalHeight });
    }
  };
  image.addEventListener("load", handleLoad);
  image.src = uri;
  return () => image.removeEventListener("load", handleLoad);
}
