# ADR-0015 Questions on the Watchtower page over Board

Status: accepted
Date: 2026-10-12
Supersedes: ADR-0013
Scope: `hiep-plugins/plugins/watchtower-board/client/**`, `hiep-plugins/plugins/board/**`

## Context

- ADR-0013 put thread input in Board's Needs you and plan question counts on Board's plan line. Board now has neither (ADR-0014).
- On the Watchtower page, Answer and Use default send a prompt to the newest idle agent of the workspace. On live on 2026-10-12 that was a mock test agent, and its name was plain text, not a link.
- `watchtower/QUESTIONS.md` does not record which thread asked, so no page can open the asking session.

## Decision

- The Watchtower page owns plan questions and manual checks. A question shows its full text and default on press, and "Actions go to X" is a link that opens thread X.
- Answer targets skip mock-provider agents; with no fitting agent, the page says so instead of picking one.
- Board shows no plan questions, checks, or counts. A thread that waits for input shows its warning badge on its Board card.
- Answers stay in the thread or on the Watchtower page. Board shows no answer form.

## Consequences

- One place per kind of item: the thread for input, the Watchtower page for questions and checks.
- The asking thread is still unknown for a question; the visible target link is the guard.

## Options Considered

- Questions and checks in Board's Needs you: rejected. The item opened a page that repeated it, cut short, with no session link.
- Question counts on a Board plan line, ADR-0013: rejected. Board has no plan line (ADR-0014).
- Record the asking agent's ID in `QUESTIONS.md`: rejected. It needs skill, parser, and autorun changes, and autorun questions usually come from agents that have already ended.
- Answer forms on Board: rejected; answers stay in the thread or on the Watchtower page.

## Revisit If

- Questions in `QUESTIONS.md` start to carry the asking agent's ID, so an item can open the asking session.
- The owner misses a question because only the Watchtower page shows it.
