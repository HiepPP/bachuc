// This plugin typechecks without the DOM library. Declare only what this module uses.
type StyleElement = { textContent: string | null; remove(): void };
type Doc = {
  head: { appendChild(node: StyleElement): unknown };
  createElement(tag: "style"): StyleElement;
};

// Claude desktop sets assistant body text to line-height 1.5 (`.font-claude-response-body`);
// Paseo uses round(contentSize * 1.4) px. Headings and monospace surfaces keep Paseo's values.
// react-native-web stamps a line-height on each text node, so the rule targets every
// descendant and needs !important to win over Paseo's generated classes.
export const CSS =
  '[data-testid="assistant-message"] *' +
  ":not([data-pmono]):not([data-pmono] *)" +
  ':not([data-paseo-markdown-tag^="h"]):not([data-paseo-markdown-tag^="h"] *)' +
  "{line-height:1.5 !important;}" +
  // Paseo's desktop header title uses weight 300, which reads as a different face beside the
  // 400-weight tabs; Claude desktop's title is regular weight.
  '[data-testid="workspace-header-title"]{font-weight:400 !important;}';

// Every connected host evaluates its own bundle in the same document; each owns its own
// style element so one host unloading does not strip the rule from the others.
export function injectStyle(doc: Doc): () => void {
  const style = doc.createElement("style");
  style.textContent = CSS;
  doc.head.appendChild(style);
  return () => style.remove();
}
