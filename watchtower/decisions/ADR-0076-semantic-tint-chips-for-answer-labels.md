# ADR-0076 Semantic tint chips over plain colored text for GPUI answer labels

Status: accepted
Date: 2026-10-28
Supersedes: ADR-0075
Scope: `packages/desktop-gpui/crates/timeline/src/answer_labels.rs`, `packages/desktop-gpui/crates/timeline/src/lib.rs`, `packages/desktop-gpui/vendor/gpui-base/src/text/**`

## Context

- The global answer rules make agents start lines with fixed labels: "What I did:", "What happened:", "Why it matters:", "Symptom:", "Cause:", "Fix:", "Not fixed:", "Unverified:", and the recap labels "Did:", "Open:", "Need from you:". On release, 20000 recent assistant rows hold "Đã làm:" 448 times, "Còn lại:" 349, "What I did:" 65, and "Cause:" 16.
- The owner allows only colors the theme already has. Plugin themes keep the `status_*` and `tint_*` tokens of their base scheme ([crates/theme/src/custom.rs](packages/desktop-gpui/crates/theme/src/custom.rs) `Tokens::from_custom`).
- The Claude Design board "Bachuc — answer label colors (GPUI)" showed four options in light and dark. The owner first chose option 1, plain colored text ([ADR-0075](watchtower/decisions/ADR-0075-semantic-status-colors-for-answer-labels.md)), then chose option 3, tint chips, on 2026-10-28.
- The vendored text view already draws inline code as a chip: 4 px side padding and a 5 px radius in the inline flow path ([inline_flow.rs](packages/desktop-gpui/vendor/gpui-base/src/text/inline_flow.rs)).

## Decision

- A label at the start of a paragraph or a list item draws as a chip: the role `tint_*` token as background, the role `status_*` token as text, medium weight, and the inline code chip shape. This holds also when the agent did not write the label in bold. The rest of the line keeps the body color.
- Roles:
  - merged (`tint_merged`, `status_merged`): "What I did:", "What happened:", "Why it matters:", "Did:", "Những gì đã làm:", "Chuyện gì đã xảy ra:", "Vì sao quan trọng:", "Đã làm:".
  - warning (`tint_warning`, `status_warning`): "Symptom:", "Unverified:", "Open:", "Triệu chứng:", "Chưa xác nhận:", "Còn lại:".
  - danger (`tint_danger`, `status_danger`): "Cause:", "Not fixed:", "Need from you:", "Nguyên nhân:", "Chưa sửa:", "Cần bạn:".
  - success (`tint_success`, `status_success`): "Fix:", "Cách sửa:".
- Plugin themes use the same tokens with no fallback.

## Consequences

- No new color token or hex value enters the GPUI theme.
- A paragraph that starts with a label renders through the inline flow path, as a paragraph with inline code does.
- A label chip sits next to inline code chips; the hue and the body-size text tell them apart.
- A plugin theme with an unusual background may give a chip less contrast.

## Options Considered

- Plain colored text with no background: rejected. The owner chose it first, then wants the stronger scan of a chip.
- One `accent` color for every label: rejected. The hue carries no role, and in Claude dark the accent is close to `status_danger`.
- `foreground_muted` for every label: rejected. It barely differs from today.
- Plain `foreground` labels on plugin themes: rejected. The owner keeps the role colors on every theme.

## Revisit If

- A contrast check shows chip text under 4.5:1 against its chip background in a built-in theme.
- The chips crowd answers that also hold many inline code chips, as the owner reports.
- The global answer rules rename or add a label role.
