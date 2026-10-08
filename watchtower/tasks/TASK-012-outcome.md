# TASK-012 Outcome

## Outcome

Status: DONE

Changed:

- [docs/hub.md](docs/hub.md), [public-docs/cli.md](public-docs/cli.md), [public-docs/hub/daemons.md](public-docs/hub/daemons.md), and [public-docs/hub/api.md](public-docs/hub/api.md): each origin order ends with the active stored login, then the `HUB_ORIGIN_REQUIRED` error. [public-docs/cli.md](public-docs/cli.md) also quotes the error text from [packages/cli/src/commands/hub/authority.ts](packages/cli/src/commands/hub/authority.ts) and drops the sentence about the hosted default.
- [public-docs/hub/configuration/index.md](public-docs/hub/configuration/index.md): its numbered origin list lost item 4, `https://hub.paseo.sh`, and gained the same error sentence. This file is not in the spec `Files:`, but the spec Goal says no doc names the fallback, and the spec grep found it.
- Kept: the API reference links in [public-docs/hub/api.md](public-docs/hub/api.md) lines 17 and 18 (Q-004), and the hosted Hub product links in [public-docs/hub/hosted.md](public-docs/hub/hosted.md) and [public-docs/hub/index.md](public-docs/hub/index.md). They are not origin fallback lines.

Contract:

- Every hub command, `paseo hub login` included, resolves its origin through `resolveHubOrigin`. With no explicit origin, no `PASEO_HUB_URL`, and no active login, it throws `HUB_ORIGIN_REQUIRED`.

Verified:

- `rg -n "then .?https://hub\.paseo\.sh|hub\.paseo\.sh.*(fallback|default)" docs public-docs` -> no match.
- `npm run format:check:files -- docs/hub.md public-docs/cli.md public-docs/hub/daemons.md public-docs/hub/api.md public-docs/hub/configuration/index.md` -> exit 0.
- Plan Verify: the license diff check -> exit 0. The plugin diff check -> no output. No `packages/*` file changed, so typecheck and lint did not run.

Anti-goal:

- `git diff --numstat bachuc-rebrand -- docs public-docs` -> 6 added, 5 deleted, 11 changed lines.
- Result: PASS, 11 of 20.

Lessons:

- In zsh, `$VAR` does not split on spaces. A file list in one variable reaches oxfmt as one path and fails with "Expected at least one target file". Pass the paths as separate words.
