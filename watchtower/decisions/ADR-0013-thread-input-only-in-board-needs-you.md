# ADR-0013 Thread input only in Board's Needs you over plan questions on Board

Status: accepted
Date: 2026-10-12
Supersedes: -
Scope: `hiep-plugins/plugins/board/**`, `hiep-plugins/plugins/watchtower-board/client/**`

## Context

- After TASK-064, Board's Needs you listed running threads that need input, plan questions, and manual checks. A question or check opened the Watchtower page, which listed the same item again, cut at 2 lines.
- On the Watchtower page, Answer and Use default send a prompt to the newest idle agent of the workspace. On live on 2026-10-12 that was a mock test agent, "Columns check", and its name was plain text, not a link.
- `watchtower/QUESTIONS.md` does not record which thread asked, so neither page can open the asking session.
- The owner found the two-step path confusing.

## Decision

- Board's Needs you lists only running threads that wait for input; pressing one opens that thread.
- Plan questions and manual checks leave Board. The plan line shows their counts, for example `· 1 question · 2 checks`, and opens Watchtower.
- The Watchtower page owns questions and checks: a question shows its full text and default on press, and "Actions go to X" is a link that opens thread X.
- Answer targets skip mock-provider agents; with no fitting agent, the page says so instead of picking one.
- The owner chose this on 2026-10-12.

## Consequences

- One place per kind of item: the thread for input, the Watchtower page for questions and checks.
- Board's Needs you is often empty; the plan line carries the plan's counts.
- The asking thread is still unknown for a question; the visible target link is the guard.

## Options Considered

- Keep questions and checks in Board's Needs you: rejected. The item opened a page that repeated it, cut short, with no session link.
- Record the asking agent's ID in `QUESTIONS.md`: rejected. It needs skill, parser, and autorun changes, and autorun questions usually come from agents that have already ended.
- Answer forms on Board: rejected earlier; answers stay in the thread or on the Watchtower page.

## Revisit If

- Questions in `QUESTIONS.md` start to carry the asking agent's ID, so an item can open the asking session.
- The owner misses a question because Board's plan line count was not noticed.
