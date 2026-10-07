# TASK-008 Host API for a sandboxed HTML frame

Group: B (shared files with TASK-006: [docs/plugins.md](docs/plugins.md), [public-docs/plugins/reference.md](public-docs/plugins/reference.md))
Class: risky

## Brief

Goal: Plugin client code can render a self-contained HTML string in a sandboxed frame on web, Electron, and native.

Change: plugin client code cannot load an iframe or a WebView -> a plugin UI component `HtmlFrame({ html, height?, network? })`.

Design: T3 Code `apps/web/src/components/chat/HtmlRenderFrame.tsx:27` and `apps/server/src/htmlRender/HtmlRender.ts:94`.

Boundaries:

- Reuse the file preview pair: [packages/app/src/file-pane/html-preview.web.tsx](packages/app/src/file-pane/html-preview.web.tsx) (iframe with `sandbox="allow-scripts"` and a CSP) and [packages/app/src/file-pane/html-preview.tsx](packages/app/src/file-pane/html-preview.tsx) (locked-down WebView).
- Follows [ADR-0002](watchtower/decisions/ADR-0002-networked-html-replies-over-offline-only.md): a frame may use the network, but the sandbox stays `allow-scripts` only, with no `allow-same-origin`, while HTML replies need CDN libraries and API calls.
- Q-008 answered: `network: true` lets the frame load scripts, styles, and images and call `fetch` over `https:`. `network` defaults to `false`, which keeps the offline preview CSP. The file preview stays offline.
- A fixed height with inner scroll; default height 360.
- Inject the app theme as CSS variables next to `withPreviewCsp` in [packages/app/src/file-pane/html-preview-csp.ts](packages/app/src/file-pane/html-preview-csp.ts).
- Export `HtmlFrame` from [packages/plugin/src/client/ui.ts](packages/plugin/src/client/ui.ts). Re-export it from [packages/app/src/plugins/react-native/ui.ts](packages/app/src/plugins/react-native/ui.ts), the module plugins import (see the import allow list in [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts), lines 534-558).
- Keep the host thin. No publish tool or timeline logic in the host; TASK-009 owns it.

How:

- Run GitNexus `impact` on `withPreviewCsp` and the plugin UI exports.
- Add a theme-variable helper and a networked CSP variant, with tests, next to `withPreviewCsp`.
- Add an `HtmlFrame` component in the app that wraps the preview pair.
- Export it through the plugin SDK UI module and the app re-export.
- Document it in [docs/plugins.md](docs/plugins.md) and [public-docs/plugins/reference.md](public-docs/plugins/reference.md).

Files:

- [packages/plugin/src/client/ui.ts](packages/plugin/src/client/ui.ts) (SDK export)
- [packages/app/src/plugins/react-native/ui.ts](packages/app/src/plugins/react-native/ui.ts) (app re-export)
- [packages/app/src/file-pane/html-preview-csp.ts](packages/app/src/file-pane/html-preview-csp.ts) (theme variables, networked CSP variant)
- [packages/app/src/file-pane/html-preview-csp.test.ts](packages/app/src/file-pane/html-preview-csp.test.ts) (tests)
- A new `HtmlFrame` component file under `packages/app/src/plugins/react-native/`, split into `.web.tsx` and native files when needed
- [docs/plugins.md](docs/plugins.md), [public-docs/plugins/reference.md](public-docs/plugins/reference.md) (docs)

Expected result:

- A test plugin renders `HtmlFrame` with a small script -> the script runs inside the frame.
- `network: false` or no `network` -> a `fetch` inside the frame is blocked by the CSP.
- `network: true` -> an `https:` script or `fetch` inside the frame works.
- In both modes, the frame cannot read app cookies or storage, and cannot navigate the app.
- The file preview still blocks the network.
- Theme CSS variables are set inside the frame.
- `height` sets the frame height. Taller content scrolls inside the frame.

Anti-goal: The frame sandbox grants only `allow-scripts`, with 0 extra sandbox tokens; tripwire; read from a test in `html-preview-csp.test.ts` or the new component test before changes, after the component change, and at completion.

## Verify

- `npx vitest run packages/app/src/file-pane/html-preview-csp.test.ts --bail=1` -> pass, with theme-variable, offline CSP, networked CSP, and sandbox cases (anti-goal read).
- `npm run typecheck` -> exit 0.
- `npm run lint` on every changed file -> exit 0.
- Browser check (verifier or human, needs live Electron or web): a test plugin renders `HtmlFrame` -> the HTML shows and its script runs. A fetch fails without `network` and works with `network: true`.
