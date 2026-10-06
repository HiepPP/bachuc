# TASK-008 Outcome

## Outcome

Status: DONE

Changed:

- Plugin SDK: `HtmlFrame` and `HtmlFrameProps` in [packages/plugin/src/client/ui.ts](packages/plugin/src/client/ui.ts), re-exported from [packages/app/src/plugins/react-native/ui.ts](packages/app/src/plugins/react-native/ui.ts).
- [packages/app/src/file-pane/html-preview-csp.ts](packages/app/src/file-pane/html-preview-csp.ts): `PREVIEW_SANDBOX`, the networked CSP, `cssVariablesStyle`, and `withPreviewCsp(html, { network?, themeStyle? })`. With no options the output is unchanged.
- The file preview pair ([html-preview.web.tsx](packages/app/src/file-pane/html-preview.web.tsx), [html-preview.tsx](packages/app/src/file-pane/html-preview.tsx)) exposes a shared `SandboxedHtmlView`, used by `FileHtmlPreview` and `HtmlFrame`. Native gets an optional `nestedScroll`.
- New [packages/app/src/plugins/react-native/html-frame.tsx](packages/app/src/plugins/react-native/html-frame.tsx) and [html-frame-theme.ts](packages/app/src/plugins/react-native/html-frame-theme.ts).
- Tests: [html-preview-csp.test.ts](packages/app/src/file-pane/html-preview-csp.test.ts), new [html-preview-csp.browser.test.ts](packages/app/src/file-pane/html-preview-csp.browser.test.ts) and [html-frame-theme.test.ts](packages/app/src/plugins/react-native/html-frame-theme.test.ts).
- Docs: [docs/plugins.md](docs/plugins.md) and [public-docs/plugins/reference.md](public-docs/plugins/reference.md).

Contract:

- ADR-0002 and Q-008: `network: true` allows `https:` scripts, styles, images, and fetch. It also allows `https:` fonts, because CDN stylesheets load them; this goes slightly past the ADR's list. `network` defaults to false, and the file preview stays offline.
- The sandbox stays `allow-scripts` only; the native WebView keeps every flag of the file preview.
- Height is fixed, 360 by default, and taller content scrolls inside. The frame background stays white; pages style themselves with 11 `--paseo-*` variables. A theme change reloads the page.
- A plugin that passes no `html` gets an empty frame instead of a crash.

Review (reviewer agent, claude-opus-5-5 xhigh): no sandbox or CSP regression found; 1 medium and 5 low findings, plus 1 likely device issue. Fixed:

- Medium: the sandbox tripwire tested only the constant. The browser test now renders the real `HtmlFrame` and checks its sandbox attribute.
- Low: the offline case now asserts a `connect-src` violation for the probed URL.
- Low: the `withUnistyles` mapping returns a style string, so a re-render with the same theme rebuilds nothing.
- Low: a missing `html` renders an empty frame.
- Low: `cssVariablesStyle` rejects `/*` and unbalanced quotes or brackets; a test proves the real light and dark font stacks still pass.
- Low: docs now say native isolation comes from the WebView flags, the background is white, a theme change reloads, and fonts are allowed.
- Likely: Android inner scroll needed `nestedScrollEnabled`; `HtmlFrame` now sets it. Not checked on a device.

Verified:

- From `packages/app`: `npx vitest run src/plugins/react-native/html-frame-theme.test.ts src/file-pane/html-preview-csp.test.ts src/file-pane/html-preview-navigation.test.ts --bail=1` -> 3 files, 15 passed.
- From `packages/app`: `npx vitest run --project browser src/file-pane/html-preview-csp.browser.test.ts --bail=1` -> 2 passed. In Chromium, the real `HtmlFrame` runs the page script. Offline, the fetch fails with a `connect-src` violation; networked, it has no violation. Storage is blocked in both.
- `npm run typecheck` -> exit 0. `npm run lint` on the changed files -> 0 warnings, 0 errors.
- GitNexus impact: `withPreviewCsp` LOW, `FileHtmlPreview` UNKNOWN with one caller (`pane.tsx`). Detect-changes: 0 affected processes.
- Live browser check with a test plugin -> UNVERIFIED (autonomous run). The live daemon on port 6768 was stopped.
- Native checks (inner scroll on Android, an https script and a CORS fetch from the `about:blank` base) -> UNVERIFIED (autonomous run); they need a device.

Anti-goal:

- Before changes: `html-preview.web.tsx` set `SANDBOX = "allow-scripts"`, with no test; 2026-10-07 01:50.
- After the component change: `PREVIEW_SANDBOX` test passed with exactly `["allow-scripts"]`; 01:52.
- Final: the unit test and the browser test on the real `HtmlFrame` passed: its iframe has `sandbox="allow-scripts"`, with 0 extra tokens; 02:05.
- Result: PASS.

Lessons:

- Browser tests that render app components need `vi.stubGlobal("React", React)`, because app sources compile with the classic JSX runtime there.
- The vitest Unistyles stub returns the component from `withUnistyles` and ignores the mapping; test theme mappings as pure functions.
