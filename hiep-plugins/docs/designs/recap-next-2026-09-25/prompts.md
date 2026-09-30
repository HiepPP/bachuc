# Recap and What's Next: image-generation prompts

These three concepts were generated with the built-in Codex Image Generation tool on 2026-09-25. The examples are synthetic. They are visual proposals, not implemented or browser-tested UI.

## 01-compact-strip

```text
Use case: ui-mockup.
Create a polished, realistic desktop UI concept for the ending of an assistant reply in Paseo. This is a design proposal, not a website landing page. Landscape image, approximately 1536 x 1024. Present a single substantial UI layout centered on a light neutral canvas, taking most of the width, no device frame and no app sidebar. The existing UI has oversized Markdown headings, a bullet-list Recap, and a gray prompt box. Improve information hierarchy and reduce empty vertical space while keeping readable text.
Visual language: crisp system sans-serif, near-white backgrounds, charcoal text, thin neutral borders, 10px corner radii, restrained spacing, simple 1.5px outline icons. No gradients, no glass, no decorative illustration, no green success colors or checkmarks implying that unverified work passed.
Use exactly this synthetic sample content in every design:
Heading: "Recap"
Label and value: "Branch" "main"
Label and value: "Did" "Reviewed the settings flow. No files changed."
Label and value: "Commit/push" "none"
Heading: "What's Next"
Suggestion text: "Check keyboard navigation in settings."
Reason text: "Find focus gaps before making changes."
Secondary button: "Edit"
Primary button: "Send", a dark filled button with a small upward arrow.
Buttons have comfortable 44px targets, secondary text has readable contrast, branch uses a small monospace chip. All three Recap fields must remain visible. Show one suggested prompt, not a list of unrelated tasks. Do not invent progress counts, a completion timestamp, passed checks, sent state, or live Git verification. The image is a full concept rendering, never a screenshot of code.

Design 01: Compact strip. The strongest recommendation. Small outside concept caption at top left: "01 / Compact strip". Below, render a wide, quiet, single-column assistant footer around 1300px wide. Recap uses a modest semibold 24px heading instead of a giant title. On the same top row, align a small Branch main chip and Commit/push none chip on the right, each with its label visible. Under the heading row, show the Did label and the summary sentence on one readable line. Use a thin horizontal divider with 24px spacing. Next comes What's Next as a 24px semibold heading. Under it, one wide faint-gray action card: suggestion text large at left, reason directly under in muted text, Edit and Send aligned right and vertically centered. Treat reason as supporting text, not a separate paragraph outside the card. Compact, polished, aligned. Avoid wrapping everything in a second unnecessary outer card. Keep the entire footer pleasantly dense, with no giant gap between sections.
```

## 02-split-panel

```text
Use case: ui-mockup.
Create a polished, realistic desktop UI concept for the ending of an assistant reply in Paseo. This is a design proposal, not a website landing page. Landscape image, approximately 1536 x 1024. Present a single substantial UI layout centered on a light neutral canvas, taking most of the width, no device frame and no app sidebar. The existing UI has oversized Markdown headings, a bullet-list Recap, and a gray prompt box. Improve information hierarchy and reduce empty vertical space while keeping readable text.
Visual language: crisp system sans-serif, near-white backgrounds, charcoal text, thin neutral borders, 10px corner radii, restrained spacing, simple 1.5px outline icons. No gradients, no glass, no decorative illustration, no green success colors or checkmarks implying that unverified work passed.
Use exactly this synthetic sample content in every design:
Heading: "Recap"
Label and value: "Branch" "main"
Label and value: "Did" "Reviewed the settings flow. No files changed."
Label and value: "Commit/push" "none"
Heading: "What's Next"
Suggestion text: "Check keyboard navigation in settings."
Reason text: "Find focus gaps before making changes."
Secondary button: "Edit"
Primary button: "Send", a dark filled button with a small upward arrow.
Buttons have comfortable 44px targets, secondary text has readable contrast, branch uses a small monospace chip. All three Recap fields must remain visible. Show one suggested prompt, not a list of unrelated tasks. Do not invent progress counts, a completion timestamp, passed checks, sent state, or live Git verification. The image is a full concept rendering, never a screenshot of code.

Design 02: Split panel. Small outside concept caption at top left: "02 / Split panel". Below, show one broad unified bordered panel about 1300px wide and 500px tall, divided vertically in a 38% left column and 62% right column. Left column: Recap heading at top, then three tidy label/value rows with generous line-height; Branch main small chip, Did value wraps naturally to two lines, Commit/push none neutral text. Slightly warm pale-gray fill on the left. Right column white: What's Next heading and one tiny neutral badge "Suggested". Under it, the suggestion is prominent and wraps at most once, followed by smaller reason. Edit and Send sit at bottom right of the right pane on a shared baseline. Show the two sections as a coordinated terminal reply panel, not a dashboard or two disconnected cards. Consistent 32px inner padding. No invented data. Strong column hierarchy, quiet professional app styling.
```

## 03-vertical-timeline

```text
Use case: ui-mockup.
Create a polished, realistic desktop UI concept for the ending of an assistant reply in Paseo. This is a design proposal, not a website landing page. Landscape image, approximately 1536 x 1024. Present a single substantial UI layout centered on a light neutral canvas, taking most of the width, no device frame and no app sidebar. The existing UI has oversized Markdown headings, a bullet-list Recap, and a gray prompt box. Improve information hierarchy and reduce empty vertical space while keeping readable text.
Visual language: crisp system sans-serif, near-white backgrounds, charcoal text, thin neutral borders, 10px corner radii, restrained spacing, simple 1.5px outline icons. No gradients, no glass, no decorative illustration, no green success colors or checkmarks implying that unverified work passed.
Use exactly this synthetic sample content in every design:
Heading: "Recap"
Label and value: "Branch" "main"
Label and value: "Did" "Reviewed the settings flow. No files changed."
Label and value: "Commit/push" "none"
Heading: "What's Next"
Suggestion text: "Check keyboard navigation in settings."
Reason text: "Find focus gaps before making changes."
Secondary button: "Edit"
Primary button: "Send", a dark filled button with a small upward arrow.
Buttons have comfortable 44px targets, secondary text has readable contrast, branch uses a small monospace chip. All three Recap fields must remain visible. Show one suggested prompt, not a list of unrelated tasks. Do not invent progress counts, a completion timestamp, passed checks, sent state, or live Git verification. The image is a full concept rendering, never a screenshot of code.

Design 03: Vertical timeline. Small outside concept caption at top left: "03 / Vertical timeline". Below, show a single-column assistant footer with two sections connected by a thin short vertical neutral line at the far left. Use tiny outline circle markers; no checkmarks and no numbered steps. First circle aligns with Recap heading. Under Recap, keep Did text visibly prominent, with label Did, then a compact horizontal metadata row of Branch main and Commit/push none. Second circle aligns with What's Next heading. Under What's Next, render one inset flat action card with white background and thin border, narrow muted charcoal left accent. Show suggestion text at top, reason beneath, and Edit and Send bottom right inside the card. Timeline line stops at the second marker, never an endless feed. Keep 24px section headings, balanced spacing, all fields visible, dense enough for a chat thread. This concept emphasizes continuity between what happened and the next user-controlled action without suggesting auto-execution.
```

