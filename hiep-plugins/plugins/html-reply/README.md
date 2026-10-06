# HTML reply

Paseo plugin with server and client entries. An agent calls `publish_html({ title, html })`, and
the page renders inline in that agent's thread as a timeline row.

## Tool

| Tool           | What it does                                                                                   |
| -------------- | ---------------------------------------------------------------------------------------------- |
| `publish_html` | Saves a self-contained HTML page of at most 1 MiB and adds a row with its title to the thread. |

The daemon `paseo` MCP server serves the tool. Native OpenCode and OMP agents cannot see plugin
tools.

## How it renders

- The page renders in the host `HtmlFrame` with `network: true` (ADR-0002): an iframe with
  `sandbox="allow-scripts"` on web and desktop, and a locked-down WebView on native. It may load
  `https:` scripts, styles, fonts, and images and call `fetch` over `https:`, but it cannot reach
  the app's cookies, storage, or windows.
- The frame is 360 points high, and taller content scrolls inside it.
- Style the page with the app theme variables: `--paseo-background`, `--paseo-surface`,
  `--paseo-foreground`, `--paseo-muted`, `--paseo-border`, `--paseo-accent`, `--paseo-success`,
  `--paseo-warning`, `--paseo-danger`, `--paseo-font-ui`, and `--paseo-font-mono`.

## Storage

- The page is saved as `$PASEO_HOME/plugin-data/html-reply/<id>.html`. The row keeps only the id
  and the title, so its data stays under 1 KiB; the daemon caps row data at 64 KiB.
- The row loads the page through the `html-reply.get` RPC. A missing file shows "Page not found".
- Rows are not restored after a daemon restart, but the page files stay.
- HTML over 1 MiB, or an empty page, makes the tool return an error, and no row is added.

## Host version

Needs the fork host from `t3code-port` (`requirements.paseo >=0.10.2-beta.900`) with
`server.registerTool` and `HtmlFrame` in `@getpaseo/plugin/client/ui`.

## Develop

```sh
npm install
npm test
npm run typecheck
npm run lint
npm run format
```

Install on live from the Paseo repo root:

```sh
npm run cli -- plugin install "$PWD/hiep-plugins/plugins/html-reply" --id html-reply
npm run cli -- plugin reload html-reply
```
