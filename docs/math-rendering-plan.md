# Math Rendering in Agent Chat — Implementation Plan

Render LaTeX in assistant messages: inline `$…$` / `\(…\)` and display `$$…$$` / `\[…\]`. Client-only change in `packages/app`. No daemon or protocol change.

Baseline: fork `main` at `e4366fa92` (`v0.10.0-beta.1` + 4 commits). The markdown pipeline files are identical to upstream `v0.10.1`, so this plan applies to both.

## Why not a plugin

The `math-render-spike` plugin proved `latex-to-unicode` works, but plugin client code may only import `react`, `react-native`, `zod`, `@tanstack/react-query` and `@getpaseo/plugin` entries. It cannot use `react-native-svg`, `react-native-webview` or the markdown component, and a timeline transformer that replaces `assistant_message` drops the original markdown. A plugin extension point would not lift those limits, so math has to be a first-party feature.

## Pipeline and insertion points

```
streamed text
  → useRevealedText            message.tsx:1517 (grapheme cuts, not markdown-aware: agent-stream/text-reveal.ts:79-95)
  → splitMarkdownBlocks        message.tsx:1961, utils/split-markdown-blocks.ts
  → parser per block           message.tsx:2000-2004 (streaming parser only for the last block while phase === "streaming")
  → markdown-it tokens → AST   react-native-markdown-display
  → render rules               message.tsx:1532 (assistant), renderer.tsx:495 createSharedMarkdownRules (other surfaces)
```

| ID  | Change                                        | Where                                                              | Why                                                                                                                                                                      |
| --- | --------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C1  | Register math rules on the assistant parser   | `utils/assistant-markdown-parser.ts:5-18`, before `if (streaming)` | Both the complete and the streaming parser get math. `utils/markdown-parser.ts` stays untouched: the file preview must show the bytes on disk (`renderer.tsx:69-70`).    |
| C2  | Teach the block splitter about `$$`           | `utils/split-markdown-blocks.ts:4-5` and `:78-92`                  | The splitter's own `MarkdownIt` decides which blank lines are structural. Without a `math_block` rule, `$$` with a blank line inside splits into two blocks.             |
| C3  | Add `math_inline` / `math_block` render rules | `components/message.tsx` rules map from line 1532                  | An unknown token type falls to the `unknown` rule, which renders `null` (`react-native-markdown-display/src/lib/AstRenderer.js:42-45`). The formula disappears silently. |
| C4  | Math components per platform                  | new `components/markdown/math/`                                    | See rendering strategy.                                                                                                                                                  |
| C5  | Copy markup                                   | `assistant-selection-copy/markup.ts:49`                            | Copying a selection must yield the TeX source, not rendered glyphs.                                                                                                      |

## Parser rules

Write the rules in-repo (`utils/markdown-math/index.ts`, ~100 lines) in the style of `utils/streaming-markdown/index.ts`. Do not add `markdown-it-dollarmath`: it peers on `markdown-it ^12 || ^13`, and the app has `markdown-it` 10.0.0 (`packages/app/package.json:106`).

Inline `$…$` follows Pandoc's rule, which is what keeps prices and shell variables out:

- The opening `$` is not followed by whitespace.
- The closing `$` is not preceded by whitespace and not followed by a digit.
- Also reject a closing `$` followed by a letter, so `$HOME/$USER` stays text.
- `\$` stays a literal dollar.

Register order:

- `math_inline` (for `$`) runs at its natural position. Backtick spans and fences already win: the inline scanner hits the backtick first, and fences are block rules.
- `\(…\)` and `\[…\]` rules go **before `escape`**, otherwise markdown-it turns `\(` into `(`.
- `math_block` (`$$` on its own lines, and `\[` blocks) goes before `paragraph`, with the same pattern on the splitter's parser (C2).

Unit tests: `$5 and $10`, `costs $5, $6`, `` `echo $HOME` ``, `$HOME/$USER`, `\$`, `$x^2$`, `\(a\)`, `$$` with a blank line inside, a fenced block containing `$$`.

## Rendering strategy

iOS renders each paragraph and inline span as a native `UITextView` (`components/markdown-text.ios.tsx:34-37, 83-89`). A non-text child inside it does not render reliably, so inline math on native must stay text.

| ID  | Surface      | Web / Electron                      | iOS / Android                                                                                                                |
| --- | ------------ | ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| R1  | Inline math  | KaTeX `renderToString` into the DOM | `latex-to-unicode` text span through `MarkdownTextSpan`; fall back to the raw TeX in code style when the conversion is lossy |
| R2  | Display math | KaTeX into the DOM                  | KaTeX inside a WebView, following the Mermaid fence host                                                                     |

Files:

- `components/markdown/math/index.tsx` — native: inline Unicode span, block WebView host.
- `components/markdown/math/index.web.tsx` — KaTeX DOM rendering, imports `katex/dist/katex.min.css`.
- `components/markdown/math/runtime/` — WebView HTML runtime, generated like `fence/mermaid/runtime/html.gen.ts` (`build:mermaid-runtime` in `packages/app/package.json`). Reuse the message and request-driver shape from `fence/mermaid/runtime/`.
- Add `katex` as a direct dependency. Today it only arrives through `mermaid` (`katex@0.16.47`).

Alternative for R2 if WebView count becomes a problem: MathJax (`mathjax-full`) to an SVG string rendered with `SvgXml` from `react-native-svg`. Synchronous and no WebView, but a large bundle and not selectable. Measure on Hermes before switching.

## Streaming

- An unclosed inline `$x^2` stays plain text until the closing `$` arrives, then flips to math. Accept the flip; do not add a provisional inline math rule to `streaming-markdown`.
- An unclosed `$$` block in the streaming tail renders the source in code style. When the block closes, render it. Keep the last good render while `phase === "streaming"`, the same as `fence/mermaid/render-model.ts:58, 94`.
- Run KaTeX with `throwOnError: false` and show the source on error.

## Phases

1. **Parser.** C1 + C2 + unit tests. Render rules map `math_*` to a code-styled source span so nothing disappears. Acceptance: all parser tests pass, existing `split-markdown-blocks` and `streaming-markdown` tests still pass.
2. **Web.** R1 + R2 on web, C5. Acceptance: browser check in `npm run dev:app` against the dev daemon with a fixture message containing inline, display, prices and shell code; copy returns TeX.
3. **Native inline.** R1 on iOS and Android. Acceptance: iOS selection still crosses spans; Android text stays selectable.
4. **Native display.** R2 WebView host. Acceptance: a message with 20 display formulas scrolls without blank frames; height settles without jumps after streaming completes.
5. **Package.** Build a release (`packages/desktop`) and test it opened in place, on the `6770` daemon, before installing it.

Each phase: run only the changed test files (`npx vitest run <file> --bail=1`), then `npm run typecheck` and `npm run lint`.

## Risks

| ID  | Risk                                                                    | Mitigation                                                                                          |
| --- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| P1  | `$` in prose parsed as math                                             | Pandoc rule plus letter check, test corpus, a setting to turn math off                              |
| P2  | Inline math on native is limited to Unicode                             | Fallback to raw TeX when the conversion is lossy                                                    |
| P3  | Many WebViews cost memory and cause layout jumps                        | WebView only for display math; cache measured height per source                                     |
| P4  | Streaming flips from raw TeX to rendered math                           | Keep the last good render; `throwOnError: false`                                                    |
| P5  | KaTeX CSS and fonts missing in the web/Electron bundle                  | Import the CSS from the web component; verify in Electron                                           |
| P6  | Copy returns rendered glyphs instead of TeX                             | C5                                                                                                  |
| P7  | Fork drift: `main` is 17 commits behind `v0.10.1`, and `v0.10.2` is out | Rebase onto the latest tag before starting; propose upstream as a first-party feature after phase 2 |
