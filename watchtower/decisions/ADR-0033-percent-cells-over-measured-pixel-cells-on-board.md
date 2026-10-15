# ADR-0033 Percent cells over measured pixel cells on Board

Status: accepted
Date: 2026-10-15
Supersedes: -
Scope: `hiep-plugins/plugins/board/client/page.tsx`, `hiep-plugins/plugins/board/tests/cluster-ui.test.ts`
Guard: forbid-text (panelWidth -) in hiep-plugins/plugins/board/client/page.tsx

## Context

- Board shows the subagents of a cluster in two columns when the panel is at least 406 px wide at the current Board scale.
- Each cell had a pixel width, `(panelWidth - edge * 3) / 2`. `panelWidth` came from the panel's `onLayout`.
- react-native-web reads `onLayout` from `offsetWidth`, which rounds to a whole pixel (`node_modules/react-native-web/dist/exports/UIManager/index.js`, `getRect`).
- When the real panel width was fractional and rounded up, the two cells and the gap overflowed by less than 1 px. The second cell wrapped to its own row.
- The wrap changed the Board height. That showed or hid the scroll bar and changed the width again, so the cells flipped between one and two per row. The owner saw this as constant flashing on 2026-10-15.

## Decision

- Give each cell `width: "50%"` in two columns and `"100%"` in one column. Make the gap with half-gap padding on the inner side of each cell.
- Use the measured width only to choose one or two columns, never to size a cell.

## Consequences

- Two 50% cells always fit on one row, whatever the rounding, so the layout does not flip.
- The gap sits inside the cell wrappers, so it is not part of a card's press area.
- The first frame after a cluster mounts still shows one column until `onLayout` reports a width.

## Options Considered

- Keep pixel cells and subtract a 1 px safety margin: rejected. The cells do not fill the row, and the width still depends on a rounded measurement.
- Pair rows with `flex: 1` cells: rejected. The fix works, but row keys change when a subagent leaves, so cards remount and lose hover state.

## Revisit If

- Board needs three or more columns, or columns of different widths.
- The panel width comes to sit near the 406 px threshold, where a scroll bar toggle can still switch the column count.
