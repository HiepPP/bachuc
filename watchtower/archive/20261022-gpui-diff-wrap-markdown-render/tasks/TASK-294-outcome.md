# TASK-294 outcome

Status: DONE

## Changed

- `hiep-plugins/plugins/selection-translate/` (new): `paseo-plugin.json`, `package.json`, `package-lock.json`, `tsconfig.json`, `index.server.ts`, `server/{llm,credentials,store,prompts,service}.ts`, `shared/{settings,contracts}.ts`, `tests/{service,llm}.test.ts`, `README.md`
- `hiep-plugins/README.md`: one catalog row; "Nineteen" -> "Twenty" independent plugins

## Contract

- RPC `selection.translate`: `{ text: string }` -> `{ translation: string }` on plugin `selection-translate`. The server trims, then checks 1..4000 chars (`Selection is empty`, `Selection is longer than 4000 characters`).
- Settings id `translate`, scope `host`, version 1: `provider` (`vercel` | `openrouter`, default `openrouter`), `model` (default `google/gemini-2.5-flash-lite`).
- Cache key `hash(provider, model, trimmedText)` in `$PASEO_HOME/plugin-data/selection-translate/cache.json`.
- `llm.ts`, `credentials.ts`, `store.ts` each name their prompt-translate source (ADR-0062). RPC input schema is `z.string()` only; the length check lives in the service so the error text matches the spec.

## Verified

- RED: `tests/service.test.ts` run before any source file: failed (module `../server/service` missing).
- `npm install && npm run typecheck && npm test` -> no type errors; 9 of 9 tests pass (3 service, 6 llm/credentials/prompts).
- `npm run lint` -> 0 warnings, 0 errors.
- `npm run cli -- plugin install ... --id selection-translate` on live (6768), then `plugin ls` -> `selection-translate running yes`.
- `git status --short -- hiep-plugins/plugins/selection-translate` -> only source files untracked; `node_modules` ignored; no `cache.json`, no key file.
- Live RPC check: UNVERIFIED (owner / TASK-295 screen check). The CLI `plugin` command has no RPC call subcommand.

## Anti-goal

Prompt-translate folder has 0 changed files.

- Checkpoint `git status --short -- hiep-plugins/plugins/prompt-translate`: before = empty (source: Bash, 11:25:29); at completion = empty (source: Bash, 11:27:43). PASS.

## Lessons

- Plugin settings through `settings.translate.read|write|reset` come from `registerSettings(translateSettings)`; the id is `translate`, same as prompt-translate, but the plugin id differs so the two do not clash.
- `index.server.ts` of the new plugin does not read `config.json` for its own path (prompt-translate does, for its hook); `configFile` is only for the Vercel credential.
- The live install registers the plugin on live only; release needs its own `plugin install`/`reload` by the owner.
