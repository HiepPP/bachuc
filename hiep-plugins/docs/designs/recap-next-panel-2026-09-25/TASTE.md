# Recap and What Next panel: taste definition

## Design read

Reading this as: an embedded reply panel inside a developer tool (Paseo desktop), for engineers who scan results between agent turns, with a calm product language, leaning toward native CSS, the host system font, Phosphor icons, and one accent.

The panel is product UI, not a landing page. It must look native to Paseo and never compete with the conversation around it.

## Dials

- DESIGN_VARIANCE 3: product UI; predictable alignment beats asymmetry.
- MOTION_INTENSITY 2: hover, press, and selection feedback only. No entry animation.
- VISUAL_DENSITY 5: daily-app spacing; the panel sits inside a chat column.

## Locks

- Color: neutral zinc family plus one accent, emerald. The accent marks only the forward action area: the What next icon, selection state, and focus. Past work (Recap) stays neutral.
- State without extra hues: commit status uses an icon change (commit glyph to check glyph), not a green chip.
- Shape: panel 12px, inner surfaces and controls 8px, chips 6px. No other radii.
- Theme: follow the host. Colors derive from host ink and background, so light and dark stay in one family.
- Type: host system font (SF Pro on macOS) for native fit; SF Mono for branch names and code. Weights 400, 500, 600 only. Tabular figures for counts.
- Icons: Phosphor regular (MIT), one family, one weight. No hand-drawn icons.

## Hierarchy

- Sections differ by surface, not by color: Recap sits on a recessed tint; What next sits on the panel surface.
- One icon per section header, one per chip, one per button. Rows, reasons, and group names carry no icons.
- The primary action is the ink Send button. Edit is secondary. Clear is a text button.

## Banned in this component

- Uppercase wide-tracking eyebrows (the v1 and v2 "RECAP" label).
- More than one accent hue (the v2 blue, amber, and green mix).
- Colored left bars on every surface, decorative dots, glows, gradients.
- Icons on every line (the v2 bulb on each reason, prompt glyph on each card).
- Em-dashes in any visible text.
- Cute or invented copy; labels stay functional ("Choose one", "Additional suggestions").

## Pre-flight for this panel

- One accent, used only for the forward action area.
- Radii limited to 12, 8, and 6.
- No uppercase eyebrow.
- Button text on one line at 400px width; Send contrast passes WCAG AA in both themes.
- Light, dark, and 400px narrow states reviewed.
- Hover, press, focus, disabled, and selected states present.
