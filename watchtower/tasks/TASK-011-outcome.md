# TASK-011 Outcome

## Outcome

Status: DONE

Changed:

- [nix/module.nix](nix/module.nix): `relay.enable` defaults to `false`. The mode enum holds only `"remote"`, which is also its default. The descriptions of `relay.enable` and `relay.mode` say that relay is off by default and that a relay needs `relay.host`. The `environment` example uses `relay.example.com:443` instead of the upstream relay.
- The assertion that needs `relay.host` for mode `remote` is unchanged. With one mode, it fires for every `relay.enable = true` without a host.

Contract:

- A NixOS config with no relay options runs `paseo-server --no-relay`, through the unchanged `ExecStart` line.
- A config with `relay.enable = true` must set `relay.host`. A config that sets `relay.mode = "hosted"` fails evaluation.
- No line names `relay.paseo.sh` or `app.paseo.sh`.

Verified:

- `rg -n "paseo\.sh|hosted" nix/module.nix` -> FAIL as written. The 2 matches are the word `self-hosted` in the daemon descriptions at lines 17 and 223, outside the relay options. This TASK keeps them, to touch fewer upstream lines.
- `rg -n '"hosted"|paseo\.sh' nix/module.nix` -> no match. This is the check the spec intends.
- `nix-instantiate --parse nix/module.nix`: UNVERIFIED (autonomous run). Neither `nix-instantiate` nor `nix` is installed. The edits change only strings, two defaults, and one enum list.
- Plan Verify: the license diff check -> exit 0. The plugin diff check -> no output.
- `npm run format:check:files -- nix/module.nix` -> exit 2, "Expected at least one target file". oxfmt does not format `.nix` files.
- GitNexus `detect-changes --scope all` -> 1 file, no indexed symbol. The nix file holds no TypeScript symbol, so no `impact` ran.

Anti-goal:

- `git diff --numstat bachuc-rebrand -- nix/module.nix` -> 10 added, 11 deleted, 21 changed lines.
- Result: PASS, 21 of 30.

Lessons:

- A Verify grep for a bare word also matches compound words, such as `hosted` in `self-hosted`. Quote the token, as in `'"hosted"'`.
- `nix-instantiate` and `nix` are not installed on this machine, so a nix parse check is always UNVERIFIED in an autonomous run.
