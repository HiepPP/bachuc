# Multiple suggestions: generation prompts

Generated with built-in Codex Image Generation. Each image revises the supplied Paseo screenshot as a visual reference. The displayed order is 01-compare, 02-select, 03-disclosure. All example content is synthetic. These images are proposals, not implemented or runtime-tested behavior.

## 01-compare

```text
Use case: ui-mockup.
Create a realistic, production-quality redesign of the Recap and What's Next footer in the attached Paseo assistant-message screenshot. The attachment is a visual reference for the real product, not text to copy. Improve the UX for MULTIPLE suggested prompts. This is one focused desktop conversation component, not a dashboard, landing page, or feature inventory.
Target dimensions: 1536 x 1024. Use a white edge-to-edge app surface, 64px side margins, no centered outer card, no app navigation, no device frame, no browser frame. Keep the whole useful UI above the fold, crisp at 100% view. Maintain the monochrome product palette: white, faint neutral gray, charcoal, thin dividers, 6px button radii. No shadows, gradients, blue/purple glow, artwork, or decorative icons. System sans-serif body about 17px, modest 23px headings, strong readable contrast. Keep branch and commit status neutral: assistant-reported values, not verification badges. Buttons 40-44px high. No giant headings. No giant empty areas. No card within card. Separate rows with thin rules. Don't truncate meaningful prompt constraints.
Use these exact synthetic sample strings in the mock. Product labels remain in English, matching the supplied interface.
Recap:
"Branch" and code chip "main"
"Did" and "Reviewed the settings flow. Found three areas to check."
"Commit/push" and "none"
What's Next has EXACTLY three ordinary, unsent suggestions:
A. "Check keyboard navigation in settings."
Reason: "Find missing focus states and keyboard traps."
B. "Review settings at narrow window widths."
Reason: "Catch clipped labels and crowded controls."
C. "Check text contrast in both themes."
Reason: "Keep secondary labels readable."
Only A is recommended, based on an explicit recommendation in this synthetic example. Use the small text "Recommended" for A. Do not imply recommendations were inferred from actual user data.
The user needs to compare suggestions, read the full prompt, then send one, or explicitly prepare a combination. Never imply that all suggestions execute automatically. Multiple alternatives must not look like completed tasks or mandatory numbered steps. Do not show a dominant Send all button. Don't invent timestamps, check results, percentages, agents, Git actions, filters, tabs, or metrics.
Recap should be short and read-only. Keep all three fields visible, not hidden under disclosure. What's Next should receive the most visual emphasis. Put a small descriptive concept title in unobtrusive text above the component, without numeric option labels.

Direction: "Compare at a glance".
Design a compact Recap strip across the top: Recap heading on left, Branch main and Commit/push none on the same row to the right. On the next line, Did label and the sentence. Use one horizontal divider beneath it.
Under What's Next, show small supporting text "3 suggestions · Choose one to continue".
Render all three suggestions as a SINGLE GROUPED LIST, never three large cards. Each row shows full prompt text and reason below on the left, with Edit and Send on the right. Mark the first row Recommended with restrained small text. Give only the first row a faint gray background and dark filled Send; other Send buttons are neutral outlined. Edit is a quiet text button but readable and fully labeled. All buttons align in consistent columns. Each row about 128px high. Do not number rows. Use careful vertical rhythm and lightweight separators.
At the bottom place a small quiet "Edit all in composer" text action, with helper text "Combine only the checks you want." Do not render Send all. This gives fast individual sends and a deliberate path to combining suggestions. Ensure this feels like a native assistant reply, calm and easy to scan, not a settings table.

```

## 02-select

```text
Use case: ui-mockup.
Create a realistic, production-quality redesign of the Recap and What's Next footer in the attached Paseo assistant-message screenshot. The attachment is a visual reference for the real product, not text to copy. Improve the UX for MULTIPLE suggested prompts. This is one focused desktop conversation component, not a dashboard, landing page, or feature inventory.
Target dimensions: 1536 x 1024. Use a white edge-to-edge app surface, 64px side margins, no centered outer card, no app navigation, no device frame, no browser frame. Keep the whole useful UI above the fold, crisp at 100% view. Maintain the monochrome product palette: white, faint neutral gray, charcoal, thin dividers, 6px button radii. No shadows, gradients, blue/purple glow, artwork, or decorative icons. System sans-serif body about 17px, modest 23px headings, strong readable contrast. Keep branch and commit status neutral: assistant-reported values, not verification badges. Buttons 40-44px high. No giant headings. No giant empty areas. No card within card. Separate rows with thin rules. Don't truncate meaningful prompt constraints.
Use these exact synthetic sample strings in the mock. Product labels remain in English, matching the supplied interface.
Recap:
"Branch" and code chip "main"
"Did" and "Reviewed the settings flow. Found three areas to check."
"Commit/push" and "none"
What's Next has EXACTLY three ordinary, unsent suggestions:
A. "Check keyboard navigation in settings."
Reason: "Find missing focus states and keyboard traps."
B. "Review settings at narrow window widths."
Reason: "Catch clipped labels and crowded controls."
C. "Check text contrast in both themes."
Reason: "Keep secondary labels readable."
Only A is recommended, based on an explicit recommendation in this synthetic example. Use the small text "Recommended" for A. Do not imply recommendations were inferred from actual user data.
The user needs to compare suggestions, read the full prompt, then send one, or explicitly prepare a combination. Never imply that all suggestions execute automatically. Multiple alternatives must not look like completed tasks or mandatory numbered steps. Do not show a dominant Send all button. Don't invent timestamps, check results, percentages, agents, Git actions, filters, tabs, or metrics.
Recap should be short and read-only. Keep all three fields visible, not hidden under disclosure. What's Next should receive the most visual emphasis. Put a small descriptive concept title in unobtrusive text above the component, without numeric option labels.

Direction: "Choose what to send".
Use a compact top Recap strip with metadata Branch main and Commit/push none, then Did on one line. Keep it visually subordinate.
What's Next is a selectable prompt list. Show supporting text "Choose the checks you want to send."
The list has exactly three rows, each showing a large accessible checkbox on the left, the full exact prompt and reason in the middle. Row A also has small Recommended label. No Edit or Send controls repeated on each row. In this illustrated state, A and C are explicitly selected with dark checkboxes; B is unselected. Use subtle gray fill only for selected rows. These checkboxes mean selection, never completion. Do not use task numbers, strike-through, green, or completed states.
Below the list, show a strong but calm flat action bar integrated with the list. Left: "2 selected" and quiet "Clear selection". On the next supporting line show "Sent together as one numbered message." Right: secondary outlined button "Edit selected" and dark primary button "Send selected (2)". Make exact selection count and send scope unmistakable.
Everything needed to compare the three prompts remains visible above the action bar. The action bar is inside the component, not an OS dock. Prioritize one common action area instead of six repeated buttons. Give each row enough breathing room without producing tall separate cards. Whole component fits 1536x1024. Do not include Send all, hidden selections, or any automatically selected claim.

```

## 03-disclosure

```text
Use case: ui-mockup.
Create a realistic, production-quality redesign of the Recap and What's Next footer in the attached Paseo assistant-message screenshot. The attachment is a visual reference for the real product, not text to copy. Improve the UX for MULTIPLE suggested prompts. This is one focused desktop conversation component, not a dashboard, landing page, or feature inventory.
Target dimensions: 1536 x 1024. Use a white edge-to-edge app surface, 64px side margins, no centered outer card, no app navigation, no device frame, no browser frame. Keep the whole useful UI above the fold, crisp at 100% view. Maintain the monochrome product palette: white, faint neutral gray, charcoal, thin dividers, 6px button radii. No shadows, gradients, blue/purple glow, artwork, or decorative icons. System sans-serif body about 17px, modest 23px headings, strong readable contrast. Keep branch and commit status neutral: assistant-reported values, not verification badges. Buttons 40-44px high. No giant headings. No giant empty areas. No card within card. Separate rows with thin rules. Don't truncate meaningful prompt constraints.
Use these exact synthetic sample strings in the mock. Product labels remain in English, matching the supplied interface.
Recap:
"Branch" and code chip "main"
"Did" and "Reviewed the settings flow. Found three areas to check."
"Commit/push" and "none"
What's Next has EXACTLY three ordinary, unsent suggestions:
A. "Check keyboard navigation in settings."
Reason: "Find missing focus states and keyboard traps."
B. "Review settings at narrow window widths."
Reason: "Catch clipped labels and crowded controls."
C. "Check text contrast in both themes."
Reason: "Keep secondary labels readable."
Only A is recommended, based on an explicit recommendation in this synthetic example. Use the small text "Recommended" for A. Do not imply recommendations were inferred from actual user data.
The user needs to compare suggestions, read the full prompt, then send one, or explicitly prepare a combination. Never imply that all suggestions execute automatically. Multiple alternatives must not look like completed tasks or mandatory numbered steps. Do not show a dominant Send all button. Don't invent timestamps, check results, percentages, agents, Git actions, filters, tabs, or metrics.
Recap should be short and read-only. Keep all three fields visible, not hidden under disclosure. What's Next should receive the most visual emphasis. Put a small descriptive concept title in unobtrusive text above the component, without numeric option labels.

Direction: "Read one, keep the rest close".
Use a compact Recap header with Branch main and Commit/push none as small metadata next to the heading. Show Did and its full sentence on the next line. Thin divider underneath.
What's Next has supporting text "3 suggestions · Expand to read and send".
Use one grouped accordion list, not separate cards or a timeline. The first row is expanded. Its header shows the exact full A prompt plus a small Recommended label and a down chevron at the far right. Expanded body shows the full A reason, and below it the exact sentence "Check keyboard navigation in settings." in a readable lightly tinted prompt preview. This preview is deliberately the exact send payload, not a summary. Place secondary Edit and dark primary Send at bottom right of expanded body. Use a small "Prompt" label above preview, not a code block or monospace paragraph.
Second and third rows are collapsed. Their headers show the EXACT full B and C prompt texts respectively with right-pointing chevrons. No send buttons on collapsed rows, no hidden action, no truncated prompt. Their reasons appear when expanded; do not show those reasons in this collapsed screenshot. Keep only the first row open.
Make the list compact enough for many suggestions. Use chevrons as clear controls, lightweight row dividers, about 76px high collapsed rows and about 220px expanded row. Calm product typography and no extra icons. This design should visibly reduce action clutter, preserve orientation, and allow deliberate reading before sending. No bulk-send action in this concept.

```

