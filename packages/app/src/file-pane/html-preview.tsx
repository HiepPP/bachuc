import { useCallback, useMemo, useRef } from "react";
import { StyleSheet } from "react-native-unistyles";
import { WebView, type WebViewMessageEvent } from "react-native-webview";
import { withPreviewCsp } from "./html-preview-csp";
import { htmlPreviewNavigationKind } from "./html-preview-navigation";

// A preview is a viewer, not a browser. Only the document Paseo hands the WebView
// loads; navigations the page attempts afterwards are refused, so a link, a
// `location.href` assignment, or a meta refresh cannot pull a remote page into the
// pane or leak the file through a URL. Storage and cache stay off so a page leaves
// nothing behind between opens.
//
// This guard is not absolute, and SECURITY.md says so rather than implying
// otherwise: the decision runs in app JS, and Android's WebView allows a
// navigation whose decision doesn't return in time. A stalled JS thread is
// therefore a window, which is why the CSP does the load-bearing work and this
// guard narrows what's left.
//
// `originWhitelist: ["*"]` is what makes that guarantee hold. react-native-webview
// checks the whitelist *before* calling onShouldStartLoadWithRequest and hands
// anything that fails it to `Linking.openURL` — so a narrow whitelist would route
// custom schemes straight to the system browser without this guard ever seeing
// them. Matching everything forces every scheme through the callback below.
const ORIGIN_WHITELIST = ["*"];

// Pinning the base URL is what makes the guard below sound. Android loads
// `source={{ html }}` through `loadDataWithBaseURL`, and a programmatic load is not
// reliably reported to onShouldStartLoadWithRequest — so the latch may still be
// unset when the page makes its first move. Naming an inert base means the only
// URLs that can pass as "initial" are inert ones. `data:text/html` must NOT be
// allowed here: a page could navigate itself to a data document of its own, which
// would arrive with no injected policy and a clean slate to egress from.
const BASE_URL = "about:blank";

export interface SandboxedHtmlViewProps {
  /** A document from `withPreviewCsp`. */
  document: string;
  /** Used by the web frame; a WebView has no title. */
  title: string;
  testID?: string;
  /** Lets the page scroll inside a scrolling parent on Android, such as a timeline row. */
  nestedScroll?: boolean;
  /**
   * Strings the page posts through `window.ReactNativeWebView`. The bridge exists only when this
   * is set, so the file preview never exposes it.
   */
  onFrameMessage?: (data: unknown) => void;
}

/**
 * The locked-down WebView shared by the file preview and plugin HTML frames. The guard governs
 * navigations only; subresource loads are left to the document's CSP.
 */
export function SandboxedHtmlView({
  document,
  testID,
  nestedScroll,
  onFrameMessage,
}: SandboxedHtmlViewProps) {
  const source = useMemo(() => ({ html: document, baseUrl: BASE_URL }), [document]);
  const handleMessage = useCallback(
    (event: WebViewMessageEvent) => onFrameMessage?.(event.nativeEvent.data),
    [onFrameMessage],
  );
  // Latched per document rather than once for the lifetime of the WebView: the
  // file pane re-renders with new content on every live-file refresh, and each of
  // those is a fresh initial load that has to be allowed through.
  const loadedDocumentRef = useRef<string | null>(null);
  const allowOnlyInitialDocument = useCallback(
    ({ url }: { url: string }) => {
      const navigationKind = htmlPreviewNavigationKind(url);
      if (navigationKind === "fragment") return true;
      if (navigationKind === "blocked") return false;
      if (loadedDocumentRef.current === document) return false;
      loadedDocumentRef.current = document;
      return true;
    },
    [document],
  );

  return (
    <WebView
      testID={testID}
      style={styles.webview}
      source={source}
      originWhitelist={ORIGIN_WHITELIST}
      onShouldStartLoadWithRequest={allowOnlyInitialDocument}
      setSupportMultipleWindows={false}
      javaScriptCanOpenWindowsAutomatically={false}
      domStorageEnabled={false}
      thirdPartyCookiesEnabled={false}
      cacheEnabled={false}
      nestedScrollEnabled={nestedScroll}
      onMessage={onFrameMessage ? handleMessage : undefined}
      incognito
    />
  );
}

export function FileHtmlPreview({ html, testID }: { html: string; testID?: string }) {
  const document = useMemo(() => withPreviewCsp(html), [html]);
  return <SandboxedHtmlView document={document} title="" testID={testID} />;
}

const styles = StyleSheet.create(() => ({
  webview: {
    flex: 1,
    backgroundColor: "white",
  },
}));
