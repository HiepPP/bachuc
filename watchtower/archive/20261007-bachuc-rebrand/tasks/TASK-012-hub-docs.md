# TASK-012 Hub docs without the hosted fallback

Group: K (writes [docs/hub.md](docs/hub.md), [public-docs/cli.md](public-docs/cli.md), [public-docs/hub/daemons.md](public-docs/hub/daemons.md), and [public-docs/hub/api.md](public-docs/hub/api.md))
Class: docs

## Brief

Goal: The hub docs state the origin rule that TASK-005 shipped. No doc says that `hub.paseo.sh` is the fallback.

Change: "explicit origin, `PASEO_HUB_URL`, active stored login, then `https://hub.paseo.sh`" -> the same list without the hosted fallback, then the `HUB_ORIGIN_REQUIRED` error.

Boundaries:

- Follows [ADR-0005](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md) and the shipped TASK-005 code in [packages/cli/src/commands/hub/authority.ts](packages/cli/src/commands/hub/authority.ts).
- Read the error text from the code. Do not copy it from this spec.
- Change only the lines about the origin fallback. Keep the links to the upstream API reference in [public-docs/hub/api.md](public-docs/hub/api.md) lines 17 and 18. They are docs links, which Q-004 keeps.
- Follow the doc voice in the root [CLAUDE.md](CLAUDE.md).

How:

- Run `rg -n "hub\.paseo\.sh" docs public-docs` and read each match.
- Rewrite each fallback line to the shipped rule.

Files:

- [docs/hub.md](docs/hub.md), [public-docs/cli.md](public-docs/cli.md), [public-docs/hub/daemons.md](public-docs/hub/daemons.md), [public-docs/hub/api.md](public-docs/hub/api.md) (origin fallback lines)

Expected result:

- Each doc that lists the origin order ends with the error, not with `hub.paseo.sh`.

Anti-goal: changed doc lines stay at or below 20; drift gauge; read with `git diff --numstat bachuc-rebrand -- docs public-docs` at completion.

## Verify

- `rg -n "then .?https://hub\.paseo\.sh|hub\.paseo\.sh.*(fallback|default)" docs public-docs` -> no match.
- `npm run format:check:files -- docs/hub.md public-docs/cli.md public-docs/hub/daemons.md public-docs/hub/api.md` -> exit 0.
- Anti-goal: `git diff --numstat bachuc-rebrand -- docs public-docs` -> 20 or less.
