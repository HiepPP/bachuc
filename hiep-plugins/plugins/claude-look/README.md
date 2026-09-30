# Claude Look

A client-only Paseo plugin that makes Paseo look like Claude desktop in light mode:

- **Claude Light** theme in **Settings → Appearance → Theme**.
- Assistant reply text at line-height 1.5, matching Claude's `.font-claude-response-body`.
  Paseo's own prose line-height is `round(contentSize * 1.4)` px and has no setting.
- Chat column narrowed from 820px to 768px, Claude's thread width.

No server entry, RPC, filesystem access, or network requests.

## Theme colors

Sources: Claude desktop 2.16120.0, `[data-theme=claude][data-mode=light]` tokens in
`Claude.app/Contents/Resources/ion-dist/assets/v1/*.css`, resolved from `--cds-hsl-*`.
Values were then compared with pixels sampled from screenshots of both running apps
(2026-09-30; JPEG captures, so ±1–2 per channel).

| Paseo key         | Hex       | Claude token              | Claude pixel | Paseo pixel |
| ----------------- | --------- | ------------------------- | ------------ | ----------- |
| `background`      | `#FCFCFB` | gray-10, conversation     | `#FCFCFC`    | `#FCFCFC`   |
| `control`         | `#F9F9F7` | bg-100 (gray-20), sidebar | `#FAFAFA`    | `#F9F9F7`   |
| `raised`          | `#FFFFFF` | bg-000, composer, menus   | `#FFFFFF`    | `#FFFFFF`   |
| `border`          | `#E7E6E1` | bg-400 (gray-80)          | see below    | `#E7E6E4`   |
| `foreground`      | `#131313` | text-100 (gray-860)       |              |             |
| `mutedForeground` | `#7B7974` | text-400 (gray-450)       |              |             |
| `accent`          | `#256ABF` | accent-100 (blue-500)     |              |             |
| `ring`            | `#A5A49A` | gray-300                  |              |             |

Paseo expands eight keys into its own tokens, so some Claude surfaces are compromises:

- `border` also paints the selected sidebar row. Claude's borders render `#E3E3E3` and its
  selected row `#EDECE8`; `#E7E6E1` sits between them.
- `raised` also paints sidebar hover rows, so hover is lighter than the sidebar. Claude's
  hover is darker.
- Claude's send button is monochrome; its blue appears on checkmarks, links, and focus.
  Paseo uses `accent` for buttons too, so its accent buttons turn blue.
- Claude draws a 1px divider between sidebar and conversation; Paseo's layout has none.

## Content width

Paseo caps chat rows, the composer, turn footers, callouts, and the markdown preview at
`MAX_CONTENT_WIDTH` (820px) through generated style classes, not stable attributes. The
plugin scans loaded stylesheets for rules with `max-width: 820px`, overrides them to 768px,
and rescans once a second when the rule count changes. Inline `max-width: 820px` styles are
covered by an attribute rule.

768px comes from screenshots of Claude desktop, calibrated by its 24px line pitch: its
composer measured about 755–770 CSS px, consistent with Tailwind `max-w-3xl` (48rem).
Claude's thread UI loads from claude.ai, so its CSS is not in the local bundle.

Paseo still estimates row heights at 820px, so long histories can shift slightly while
scrolling into rows that have not been measured yet.

## Typography

The injected rule targets text inside `[data-testid="assistant-message"]` on web clients
(desktop app and browser). It skips headings (`data-paseo-markdown-tag="h1"`…`"h6"`) and
monospace surfaces (`data-pmono`: code blocks, diffs, terminal). User messages, the sidebar,
and native clients are unchanged. The selectors are Paseo DOM attributes, not a public API;
if Paseo renames them the rule becomes a no-op.

Use **Settings → Appearance** for the values Paseo already exposes:

| Claude desktop                                                   | Paseo setting               |
| ---------------------------------------------------------------- | --------------------------- |
| Chat font "System": `ui-sans-serif, system-ui, -apple-system, …` | Interface font: leave empty |
| Response body `1rem` (16px)                                      | Content size: 16            |
| Code `.813rem` (13px)                                            | Code size: 13               |

Claude's lighter regular weight (`"wght" 360`) applies only in dark mode, so light mode
already matches Paseo's 400.

## Install

Requires Paseo 0.10.1 or later.

```sh
cd plugins/claude-look
npm install
npm run typecheck
npm run lint
paseo plugin install "$PWD" --id claude-look
paseo plugin ls claude-look --json
```

Then choose **Claude Light** in Settings → Appearance → Theme.
After edits: `npm run format`, typecheck, lint, then `paseo plugin reload claude-look`.
