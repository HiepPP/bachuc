# Revised selection design: exclusive alternatives

Generated with built-in Codex Image Generation from the selected version 2 reference. Synthetic example data only. This is a visual proposal; the plugin has not been changed.

Implementation boundary: radio groups enforce declared alternatives, not general semantic compatibility. Without reliable grouping, default to one selected prompt. Allow manual combination through the composer. Any later implementation must enforce selection constraints in the send handler as well as the UI. Do not infer compatibility merely from missing negation words.

```text
Use case: ui-mockup.
Edit the attached selected Paseo concept into a safer, clearer multi-suggestion interface. The attachment is the selected visual target. Preserve its monochrome system typography, white edge-to-edge desktop surface, thin borders, compact Recap strip, and shared bottom action bar. Target 1536 x 1024. Render exactly ONE revised concept, not a comparison board. No app frame, decorative art, gradients, shadows, new navigation, or success badges.
The user identified a real UX flaw: two suggestions can contradict each other, such as implementing a change versus explicitly not implementing it. Never offer independent checkboxes for those two alternatives.
Use this improved interaction: mutually exclusive approaches use RADIO BUTTONS within one explicitly labelled group; a compatible supplemental step uses a CHECKBOX in a separate group. No automatic conflict detection, AI verification badge, or claim that textual similarity proves compatibility. These are explicitly declared relationships in the example. Show exactly three suggestions total. One selected approach and one selected supplemental step = 2 selected. Clearly draw radio controls as circles with dots; draw checkbox as a square with a tick. The excluded alternative is unselected, still readable and clickable, not crossed out, not disabled.
Exact visible sample copy:
Top small concept label: "Choose an approach, then add detail"
Heading: "Recap"
"Branch:" and code chip "main"
"Did:" and "Reviewed the next steps. No code changed."
"Commit/push:" and "none"
Thin separator.
Heading: "What's Next"
Supporting line: "Choose one approach. Add a compatible follow-up if needed."
Group heading: "Approach" and helper "Choose one"
First RADIO, selected:
"Implement the settings redesign."
Reason: "Apply the agreed layout changes."
Second RADIO, unselected:
"Do not implement the redesign. Review it only."
Reason: "Identify issues without changing code."
Small contextual note below the two radios:
"These approaches cannot be combined."
This should be a neutral explanation, not a scary warning banner. Place the two options together within one calm divided group surface. Selected approach gets a very faint gray row tint.
Below it a separate compact group heading: "Optional follow-up"
One CHECKBOX, selected:
"List remaining questions and risks."
Reason: "Include this after the chosen approach."
Shared bottom action bar:
Left: "2 selected"
Supporting line: "One approach + one follow-up"
Right: outlined "Edit selected" and primary charcoal "Send selected (2)"
Make the send payload scope unmistakable. Do not show Send all. Do not show two selected radios. Do not claim live compatibility checks or present implementation as completed. All exact text must be readable, no clipped labels.
Keep 64px horizontal margins, 22-24px headings, 17-18px prompt text, 15-16px secondary text, 40-44px controls. Keep the entire component comfortably above the fold. Use layout and typography rather than excessive nested cards. This is a revised visual proposal, not working UI.
```

