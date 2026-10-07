// A preview renders a self-contained document and nothing else. Inline styles and
// scripts run so a plan page keeps its layout and its diagrams; fetch, XHR,
// WebSocket, beacon, remote script, remote font, remote image, and form posts are
// all refused. Agent-written HTML is not trusted markup.
//
// What this does NOT stop: the document navigating itself. No CSP directive
// available in current browsers prevents it — `navigate-to` was dropped from CSP3
// and is unenforced, and `<meta http-equiv="refresh">` needs no script at all
// (both verified against the Chromium this app ships against). So a hostile page
// can still reach a server by navigating, carrying data available inside the
// preview. The opaque origin is what bounds the damage: the frame has no storage,
// no parent access, and no way to read any file but itself. Native narrows it
// further in html-preview.tsx, because a WebView can refuse navigation outside CSP — see the
// caveat there on why that is a mitigation rather than a guarantee.
const POLICY = [
  "default-src 'none'",
  "script-src 'unsafe-inline' 'unsafe-eval' blob:",
  "style-src 'unsafe-inline'",
  "img-src data: blob:",
  "font-src data:",
  "media-src data: blob:",
  "connect-src 'none'",
  "form-action 'none'",
  "base-uri 'none'",
  "frame-src 'none'",
  "object-src 'none'",
].join("; ");

// Plugin HTML frames may opt into the network (ADR-0002): https scripts, styles, fonts, images,
// and fetch. Everything else stays refused, and the file preview never uses this policy. The
// sandbox below, not this policy, is what keeps the page away from the app.
const NETWORK_POLICY = [
  "default-src 'none'",
  "script-src 'unsafe-inline' 'unsafe-eval' blob: https:",
  "style-src 'unsafe-inline' https:",
  "img-src data: blob: https:",
  "font-src data: https:",
  "media-src data: blob:",
  "connect-src https:",
  "form-action 'none'",
  "base-uri 'none'",
  "frame-src 'none'",
  "object-src 'none'",
].join("; ");

function meta(policy: string): string {
  return `<meta http-equiv="Content-Security-Policy" content="${policy}">`;
}

const META = meta(POLICY);

/**
 * `allow-scripts` alone gives the document an opaque origin: its scripts run, but it cannot
 * reach the app's DOM, cookies, or storage, open popups, or navigate the top window.
 */
export const PREVIEW_SANDBOX = "allow-scripts";

const CSS_VARIABLE_NAME = /^[a-z][a-z0-9-]*$/;
// A value may hold a font stack with quotes, but nothing that ends the declaration, the rule,
// or the style element, and no comment or open quote or bracket that would swallow the
// declarations after it.
const UNSAFE_CSS_VALUE = /[<>{};\\]|\/\*/;

function count(value: string, character: string): number {
  return value.split(character).length - 1;
}

function isSafeCssValue(value: string): boolean {
  return (
    !UNSAFE_CSS_VALUE.test(value) &&
    count(value, '"') % 2 === 0 &&
    count(value, "'") % 2 === 0 &&
    count(value, "(") === count(value, ")")
  );
}

/** `:root` custom properties as a style element, skipping any unsafe name or value. */
export function cssVariablesStyle(variables: Readonly<Record<string, string>>): string {
  const declarations = Object.entries(variables)
    .filter(([name, value]) => CSS_VARIABLE_NAME.test(name) && isSafeCssValue(value))
    .map(([name, value]) => `--${name}: ${value};`)
    .join(" ");
  return declarations ? `<style>:root { ${declarations} }</style>` : "";
}

export interface PreviewDocumentOptions {
  /** Allows https resources and fetch. Only plugin HTML frames set it. */
  network?: boolean;
  /** A style element from `cssVariablesStyle`, placed before the document's own markup. */
  themeStyle?: string;
  /** Posts the content height to the host, so the frame can grow to fit the page. */
  reportHeight?: boolean;
}

export const FRAME_HEIGHT_MESSAGE_KEY = "paseoHtmlFrameHeight";

// A sandboxed frame has an opaque origin, so the host cannot read its height; the page reports
// it instead, through postMessage on web and the WebView bridge on native. It measures the root
// box plus the body's margins, which collapse outside it, and never `scrollHeight`, which is at
// least the frame's own height and so could never shrink. The page is untrusted: the host clamps
// whatever arrives.
const HEIGHT_REPORTER = `<script>(function () {
  var last = -1;
  function send() {
    var body = document.body;
    if (!body) return;
    var style = getComputedStyle(body);
    var height = Math.ceil(document.documentElement.getBoundingClientRect().height +
      parseFloat(style.marginTop) + parseFloat(style.marginBottom));
    if (Math.abs(height - last) < 1) return;
    last = height;
    var message = { ${FRAME_HEIGHT_MESSAGE_KEY}: height };
    if (window.ReactNativeWebView) window.ReactNativeWebView.postMessage(JSON.stringify(message));
    else parent.postMessage(message, "*");
  }
  function start() {
    send();
    if (window.ResizeObserver) {
      var observer = new ResizeObserver(send);
      observer.observe(document.documentElement);
      observer.observe(document.body);
    }
    window.addEventListener("load", send);
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start);
  else start();
})();</script>`;

// The policy must reach the parser before any markup the document declares, and it
// only counts if it lands in `<head>` — once the parser has moved on to `<body>`, a
// meta http-equiv CSP is ignored outright.
//
// Locating the document's own doctype to insert after it means reimplementing the
// tokenizer's initial insertion mode: its exact whitespace set (JS `\s` matches
// characters HTML does not, and one stray NBSP is enough to push the policy into
// the body where it stops applying), every comment ending including `--!>`, `<!-->`
// and `<!--->`, bogus-comment tokens like `<?xml …?>` and `<![CDATA[…]]>`, and the
// rule that a doctype closes at the first `>` in every state. Each of those rules
// cost a bug before it was right.
//
// So the prologue isn't found, it's supplied: our doctype, then the policy, then
// the file verbatim. The file's own doctype becomes a stray DOCTYPE token, which
// the parser ignores wherever it appears. Standards mode is guaranteed, the policy
// is always the first element and therefore always in the head, and no part of the
// document has to be parsed to place it.
const PROLOGUE = `<!doctype html>${META}`;

// Left where it is, a BOM would sit mid-document and render as a zero-width space.
const BOM = "\uFEFF";

export function withPreviewCsp(html: string, options: PreviewDocumentOptions = {}): string {
  const prologue = options.network ? `<!doctype html>${meta(NETWORK_POLICY)}` : PROLOGUE;
  const reporter = options.reportHeight ? HEIGHT_REPORTER : "";
  return (
    prologue +
    (options.themeStyle ?? "") +
    reporter +
    (html.startsWith(BOM) ? html.slice(BOM.length) : html)
  );
}
