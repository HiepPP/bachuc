// Claude desktop light-mode tokens (`[data-theme=claude][data-mode=light]`), resolved from
// `--cds-hsl-*` and checked against pixels sampled from the running app. See README.
export const claudeLight = {
  id: "claude-light",
  name: "Claude Light",
  appearance: "light",
  colors: {
    background: "#FCFCFB", // gray-10: conversation pane
    foreground: "#131313", // text-100 (gray-860)
    raised: "#FFFFFF", // bg-000: composer, popovers, hovered rows
    control: "#F9F9F7", // bg-100 (gray-20): sidebar, inputs
    border: "#E7E6E1", // bg-400 (gray-80): borders and the selected sidebar row
    accent: "#256ABF", // accent-100 (blue-500): checkmarks, focus, selection
    mutedForeground: "#7B7974", // text-400 (gray-450)
    ring: "#A5A49A", // gray-300: scrollbars, extra-muted text
  },
} as const;
