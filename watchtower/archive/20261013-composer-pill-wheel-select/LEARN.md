# Learn 20261013-composer-pill-wheel-select

## Summary

Discrepancy: 4 found. The four TASKs shipped as planned, but the owner's live tries replaced the plan's core rule: the wheel now opens the list, a hover opens it too, and the step model changed.

## Per TASK

- TASK-079: plan a 48 px step helper with one step per event, and a web hook in `use-wheel-select.web.ts` -> shipped as planned, then replaced. The listener moved to [packages/app/src/hooks/use-wheel-listener.web.ts](packages/app/src/hooks/use-wheel-listener.web.ts), and the step became one step for the first event of a gesture plus one per 40 px, with no cooldown. Mistake: the spec fixed pixel values before anyone measured a real notch (Q-032). Fix: make the first live mouse check a blocking step before tuning numbers.
- TASK-080: plan a wheel on the closed toolbar pills, with the list kept closed -> shipped as planned. Then the owner found it too fast and gave no overview. Now a step opens the list, the backdrop steps too, and a hover opens it. Mistake: the closed-list rule (ADR-0024) was never tried with a mouse before the build. Fix: when a TASK sets an interaction feel, plan a short live try first.
- TASK-081: plan the model pill wheel on the `ComboboxTrigger` branch -> match (Q-033 default held).
- TASK-082: plan one rule in [docs/menus.md](docs/menus.md) -> shipped, then rewritten twice as the behavior changed. Match for its own scope.

## Plan-Level

- The plan missed the plugin pills. Caveman and Skills did not react to the wheel, which led to a new plugin `select` behavior and a menu focus walk outside the plan's TASKs.
- Every live check stayed `UNVERIFIED (autonomous run)`. The owner's first live try found three of the four later changes.
- Q-031 (pending local value) and Q-032 (notch size) are now resolved in code, but their rows stay `DEFAULTED`.

## Lessons

- For a feel-driven feature, put one owner live try before the autonomous build.
- Ask which plugin surfaces a host interaction must cover when the composer mixes host and plugin pills.
- Record the final decision in one ADR and update it in place while the work is still uncommitted.
