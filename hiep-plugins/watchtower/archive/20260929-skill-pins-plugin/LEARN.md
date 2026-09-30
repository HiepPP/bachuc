# Learn 20260929-skill-pins-plugin

## Summary

Discrepancy: 5 found. The plugin shipped what the plan asked for, but three TASKs changed their
mechanism mid-flight, and the plan grew from four TASKs to six.

## Per TASK

- TASK-001: plan said scaffold from `paseo plugin init` -> shipped a hand-written scaffold that
  only copied the shape. Mistake: the CLI is 0.10.1 and pins SDK 0.10.1, while every other plugin
  here uses `0.9.0-beta.2`. Likely cause: the spec assumed the CLI matches the repo. Fix: state the
  SDK version in the spec instead of naming the scaffold command.
- TASK-002: match.
- TASK-003: plan said capture a snapshot on Enter -> shipped a watcher for new host queue rows.
  Mistake: `prompt-translate` already stops the host Enter and replays its own, so two capture
  listeners would race. Likely cause: the spec was written without reading that plugin's listener.
  Fix: read every sibling plugin that touches the same event before writing the spec.
- TASK-004: plan said install and confirm a live turn -> shipped that, plus a catalog swap, a UI
  redesign, two hook wording changes, and archive cleanup. Mistake: the first live turn showed the
  model skipping a pinned skill, and each fix landed in TASK-004 instead of a new TASK. Likely
  cause: the plan had no row for "make the model actually comply". Fix: when a verify run exposes a
  behavior gap, open a TASK for it.
- TASK-005: plan named `agent.created` and `shared/contracts.ts` -> shipped
  `before("agent.session_open")` with reason `create`, and a separate `shared/settings.ts`.
  Mistake: `agent.created` carries no env, so it cannot read what `agent.create` added. Likely
  cause: the event was chosen from its name, not from the SDK types. Fix: check the event payload
  before naming it in a spec.
- TASK-006: match on code. The verify step needed three runs. The first two recorded UNVERIFIED
  because a desktop check was reported but left no host evidence. Fix: verify a manual UI claim
  against host state in the same run, or drive the UI directly.

## Plan-Level

- Scope creep: the plan started at four TASKs and ended at six. The catalog was also replaced
  mid-plan, from three superpowers skills to watchtower, chase-goal-claude, and
  sequential-thinking. Neither change got its own TASK.
- Plan misdescribes reality: the Handoff said TASK-002 and TASK-003 could run in parallel. Both
  edit the `lint` and `format` scripts in `package.json`, so they had to run in order. The Handoff
  was corrected during TASK-001.
- Missing row: the real fix for the skipped-skill problem was not code in this plugin. It was the
  wording in `~/.claude/skills/sequential-thinking/SKILL.md`, a file outside this repo. No TASK
  covered work outside the repo.

## Lessons

- Read the sibling plugin that owns the same DOM event or lifecycle hook before writing a spec that
  touches it.
- Pick a lifecycle event from its payload type, not its name. `agent.created` has no env.
- A daemon can require things the pinned SDK types do not. Daemon 0.10.1 rejects a `contribute`
  that returns nothing, and typecheck against 0.9.0-beta.2 does not catch it.
- Treat "the user says the UI worked" as a claim, not evidence. Check host state in the same run.
- When the catalog or the user-visible set changes mid-plan, add a TASK. It changes tests, docs, and
  the installer together.
