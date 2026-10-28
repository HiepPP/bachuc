# ADR-0075 Semantic status colors over one accent, tint chips, or muted gray for GPUI answer labels

Status: superseded by ADR-0076
Date: 2026-10-28
Supersedes: -
Scope: `packages/desktop-gpui/crates/timeline/src/answer_labels.rs`, `packages/desktop-gpui/crates/timeline/src/lib.rs`, `packages/desktop-gpui/vendor/gpui-base/src/text/**`

## Context

- The global answer rules make agents start lines with fixed labels: "What I did:", "What happened:", "Why it matters:", "Symptom:", "Cause:", "Fix:", "Not fixed:", "Unverified:", and the recap labels "Did:", "Open:", "Need from you:". On release, 20000 recent assistant rows hold "Đã làm:" 448 times, "Còn lại:" 349, "What I did:" 65, and "Cause:" 16.
- The GPUI answer draws every label in the body color, so a reader cannot scan an answer by role.
- The owner allows only colors the theme already has. Plugin themes keep the `status_*` tokens of their base scheme ([crates/theme/src/custom.rs](packages/desktop-gpui/crates/theme/src/custom.rs) `Tokens::from_custom`), and `accent` follows the plugin theme.
- In Claude dark, `accent` (#d97757) is close to `status_danger` (#d8847b).
- The Claude Design board "Bachuc — answer label colors (GPUI)" showed four options in light and dark. The owner chose option 1 on 2026-10-28.

## Decision

- A label at the start of a paragraph or a list item gets a theme status color and medium weight, also when the agent did not write it in bold. The rest of the line keeps the body color.
- Colors by role:
  - `status_merged`: "What I did:", "What happened:", "Why it matters:", "Did:", "Những gì đã làm:", "Chuyện gì đã xảy ra:", "Vì sao quan trọng:", "Đã làm:".
  - `status_warning`: "Symptom:", "Unverified:", "Open:", "Triệu chứng:", "Chưa xác nhận:", "Còn lại:".
  - `status_danger`: "Cause:", "Not fixed:", "Need from you:", "Nguyên nhân:", "Chưa sửa:", "Cần bạn:".
  - `status_success`: "Fix:", "Cách sửa:".
- Plugin themes use the same tokens with no fallback.

## Consequences

- No new color token or hex value enters the GPUI theme. A label color changes when the theme changes.
- A plugin theme with an unusual background may give a label less contrast, because `status_*` comes from the base scheme.
- A new label needs one row in the label table and a role from the four above.
- `status_danger` on "Need from you:" reads as urgent, not as an error.

## Options Considered

- One `accent` color for every label: rejected. The hue carries no role, and in Claude dark the accent is close to `status_danger`.
- Tint chips (`tint_*` background with `status_*` text): rejected. It adds a pill shape next to the inline code chips and needs a background mark in the vendored text view.
- `foreground_muted` for every label: rejected. It barely differs from today and does not help a reader scan.
- Plain `foreground` labels on plugin themes: rejected. The owner keeps the status colors on every theme.

## Revisit If

- A contrast check shows a label color under 4.5:1 against the answer background in a built-in theme.
- The theme gains a categorical color family that can carry four distinct label hues.
- The global answer rules rename or add a label role.
