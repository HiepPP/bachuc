# TASK-143 Design the plugin UI host for the GPUI client

Group: I (writes only research and a proposed ADR)
Class: docs

## Brief

Goal: Decide how the GPUI client will host plugin UI, so a later plan can port the plugins.

Change: no plugin UI in the GPUI client -> a research note with measured options and a proposed ADR.

Boundaries:

- Do not port a plugin and do not write client code in this TASK.
- Plugin client entries are React bundles built by [packages/server/src/server/plugins/compiler.ts](packages/server/src/server/plugins/compiler.ts). The client contract is in [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts).
- [hiep-plugins/plugins](hiep-plugins/plugins) has 16 `index.client.tsx` entries. An earlier read found about 15 that contribute UI (pages and panels, timeline renderers, composer controls, attachment sources, and a theme). Confirm the count per plugin.
- Follows [ADR-0017](watchtower/decisions/ADR-0017-native-host-features-over-thin-host-changes.md): put a feature where it runs natively with the fewest bugs.
- Follows [ADR-0037](watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md): live WebViews do not clip inside GPUI scroll lists. Any WebView option must say how it handles this.
- Options to compare, at least: React plugin surfaces in a WebView per surface; a declarative UI contract that GPUI renders; moving each plugin's UI into the host or rewriting it in Rust.

How:

- Map each UI contribution kind to the plugins that use it, with counts.
- For each option, estimate the work per contribution kind, the memory per open surface, and the risk. Measure one WebView surface with `measure.sh` if you build a throwaway probe outside the repo.
- Write the research note and a `proposed` ADR. Do not set it to `accepted`; the owner decides.

Files:

- [watchtower/research/RESEARCH-001-plugin-ui-host.md](watchtower/research/RESEARCH-001-plugin-ui-host.md) (new research note; take the next free research ID)
- [watchtower/RESEARCH.md](watchtower/RESEARCH.md) (new index file with one row, per the watchtower research template)
- [watchtower/decisions](watchtower/decisions) (one new `proposed` ADR file with the next free ADR ID)
- [watchtower/DECISIONS.md](watchtower/DECISIONS.md) (index row with status `proposed`)

Interfaces:

- Consumes: the bundle `target/bundles/Bachuc GPUI.app` built by `bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop` from TASK-141

Expected result:

- The note has a table of contribution kinds, plugin counts, and the effort of each option.
- The proposed ADR names one recommended option, the rejected options with reasons tied to the facts, and a `## Revisit If`.

Anti-goal: no file outside [watchtower](watchtower) changes; tripwire; read with `git status --short` at completion.

## Verify

- `rg -n "^\| " watchtower/research/RESEARCH-*-plugin-ui-host.md` -> the contribution table has one row per contribution kind
- `rg -n "proposed" watchtower/DECISIONS.md` -> one new row for the plugin UI host ADR
- Anti-goal: `git status --short -- . ':!watchtower'` -> no change from this TASK
