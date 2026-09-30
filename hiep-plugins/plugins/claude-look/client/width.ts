// This plugin typechecks without the DOM library. Declare only what this module uses.
type Rule = {
  selectorText?: string;
  style?: { maxWidth?: string };
  cssRules?: ArrayLike<Rule>;
};
type Sheet = { ownerNode?: unknown; cssRules: ArrayLike<Rule> };
type StyleElement = { textContent: string | null; remove(): void };
type Doc = {
  head: { appendChild(node: StyleElement): unknown };
  createElement(tag: "style"): StyleElement;
  styleSheets: ArrayLike<Sheet>;
};
type Timers = {
  setInterval(callback: () => void, ms: number): unknown;
  clearInterval(id: unknown): void;
};

// Paseo caps chat rows, the composer, and nearby callouts at MAX_CONTENT_WIDTH (820px) through
// generated style classes, not stable attributes. Claude desktop's thread measures about
// 768px (48rem). Find every rule that sets the 820px cap and override it.
const PASEO_WIDTH = "820px";
const CLAUDE_WIDTH = "768px";
const INLINE = `[style*="max-width: ${PASEO_WIDTH}"]{max-width:${CLAUDE_WIDTH} !important;}`;

function readRules(rules: ArrayLike<Rule>, selectors: Set<string>): number {
  let count = rules.length;
  for (let i = 0; i < rules.length; i++) {
    const rule = rules[i];
    if (rule.style?.maxWidth === PASEO_WIDTH && rule.selectorText) selectors.add(rule.selectorText);
    if (rule.cssRules) count += readRules(rule.cssRules, selectors);
  }
  return count;
}

export function narrowContent(doc: Doc, timers: Timers): () => void {
  const style = doc.createElement("style");
  style.textContent = INLINE;
  doc.head.appendChild(style);
  let seen = -1;
  const scan = () => {
    const selectors = new Set<string>();
    let count = 0;
    for (let i = 0; i < doc.styleSheets.length; i++) {
      const sheet = doc.styleSheets[i];
      if (sheet.ownerNode === style) continue;
      try {
        count += readRules(sheet.cssRules, selectors);
      } catch {
        // Cross-origin sheets refuse cssRules access; they cannot hold Paseo's styles.
      }
    }
    // Style generators append rules as screens mount; rescan only when the total changes.
    if (count === seen) return;
    seen = count;
    const list = [...selectors];
    style.textContent =
      INLINE + (list.length ? `${list.join(",")}{max-width:${CLAUDE_WIDTH} !important;}` : "");
  };
  scan();
  const timer = timers.setInterval(scan, 1000);
  return () => {
    timers.clearInterval(timer);
    style.remove();
  };
}
