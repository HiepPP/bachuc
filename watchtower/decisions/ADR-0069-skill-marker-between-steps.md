# ADR-0069 Skill marker line above the step over chips, a turn strip, or only a cleaner card

Status: accepted
Date: 2026-10-27
Supersedes: -
Scope: `packages/desktop-gpui/crates/timeline/src/lib.rs`, `packages/desktop-gpui/crates/timeline/src/grouping.rs`

## Context

- After [ADR-0068](watchtower/decisions/ADR-0068-hooks-in-tool-calls-and-stored-usage-summary.md), a `Skill` call showed only inside its step body, between hook rows. In a 30-step turn, Hiep had to open every card to find it.
- Claude Design project "Bachuc — skills in the step list" showed four options: 1a chips on the step header, 1b one skill strip under "Worked for", 1c a marker line between steps, and 1d a cleaner open card with folded hooks.

## Decision

- A step that called the `Skill` tool gets one marker line above it: a sparkle on the step rail, the skill names, a rule, and the time since the turn start.
- The marker uses the theme's ANSI magenta, because the theme has no skill token.
- Hiep chose 1c on 2026-10-27, after seeing the four options.

## Consequences

- The marker is drawn inside the step row, so the step list keeps its row count.
- A skill typed as `/name` gets no marker, because it has no timeline row; the header chip counts it.
- The marker time equals the step time when the skill is the step's first call.

## Options Considered

- 1a, chips on each step header: rejected. Hiep picked the marker line.
- 1b, one strip under "Worked for": rejected. Hiep picked the marker line, which also shows the order and time of skills.
- 1d, a cleaner open card only: rejected as the main fix, because it still needs the card to be opened.

## Revisit If

- A turn with more than about 10 skills makes the step list too long.
- A slash-command skill gets its own timeline row.
