# ADR-0077 Recap as one branch line over Did, Open, and Need from you bullets

Status: accepted
Date: 2026-10-28
Supersedes: -
Scope: `hiep-plugins/plugins/board/server/recaps.ts`, `packages/desktop-gpui/plugins/next-prompt-actions/src/section.rs`, `~/.claude/CLAUDE.md`, `~/.claude/hooks/answer-ending-check.cjs`

## Context

- The global answer rules ended each task answer with a Recap of 2-4 bullets: `<branch> · <commit state>`, Did, Open, and Need from you. The body above already states the result in its bold opening, and its labeled groups hold the open items, so the Recap card repeated them (owner screenshot, 2026-10-28).
- Three parts read the Recap: the Board Recaps tab stores branch, Did, and commit per turn (`parseRecap`), the GPUI next-prompt-actions card folds it into chips and sections (`compact_recap`), and the stop hook requires it.

## Decision

- The Recap is one bullet, `<branch> · <commit state>`. Open and Need from you move into the body as labeled groups ("Open:", "Need from you:", and the Vietnamese "Còn lại:", "Cần bạn:"), which the answer label chips color.
- The Board Recaps tab takes Did from the bold opening sentence, cut at 300 characters, when the Recap has no Did. An old Recap with a Did bullet keeps it.
- The GPUI card shows only the branch and commit chips for a one-bullet Recap.
- The owner chose it on 2026-10-28.

## Consequences

- Answers lose the Did summary line; the Board Recaps tab shows the bold opening instead.
- An answer with no bold opening stores no Did.
- The stop hook flags a Recap with a second bullet.

## Options Considered

- Drop the Recap entirely: rejected. The Board Recaps tab would get no new entries, and the branch and commit state appear nowhere else.
- Keep the Recap and only forbid repeats: rejected. The repeat comes back whenever an agent does not follow the rule, and the card still shows Did.

## Revisit If

- The Board Recaps tab needs a summary that the bold opening cannot give, as the owner reports.
- The branch and commit state move into the agent header, so the Recap line itself repeats information.
