import type { AssistantImageDimensions } from "./load-dimensions";

// Native Image onLoad does not wait on a decode step, so only web needs a separate watch.
export function watchImageLoadDimensions(
  _uri: string,
  _onLoad: (dimensions: AssistantImageDimensions) => void,
): () => void {
  return () => {};
}
