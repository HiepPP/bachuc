# TASK-011 Nix relay without the upstream default

Group: J (standalone; writes [nix/module.nix](nix/module.nix) only)
Class: code

## Brief

Goal: The nix module no longer turns on relay toward the upstream service by default.

Change: `relay.enable` defaults to `true` with mode `hosted` (the upstream relay) -> relay is off by default, and a relay needs a host the owner sets.

Boundaries:

- Follows [ADR-0005](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md).
- After TASK-005, the daemon has no default relay endpoint. Mode `hosted` now means relay off, so its description in [nix/module.nix](nix/module.nix) lines 92 to 98 is wrong.
- Line 162 sets `PASEO_RELAY_ENDPOINT = "relay.paseo.sh:443"`. Read its context first.
- Smallest change: default `relay.enable` to `false`. Remove mode `hosted` and the upstream endpoint, so mode `remote` with `relay.host` is the only relay path. Keep the assertion that needs `relay.host` for mode `remote`.
- Update the option descriptions to match.

How:

- Read [nix/module.nix](nix/module.nix) lines 80 to 170 and 200 to 280.
- Change the defaults and the mode enum. Remove the upstream endpoint. Update the descriptions.
- Parse the file, as in Verify.

Files:

- [nix/module.nix](nix/module.nix) (relay defaults, mode enum, descriptions)

Expected result:

- A NixOS config with no relay options runs the daemon with `--no-relay`.
- A config with `relay.enable = true` must set mode `remote` and `relay.host`.
- No line names `relay.paseo.sh` or `app.paseo.sh`.

Anti-goal: changed lines in [nix/module.nix](nix/module.nix) stay at or below 30; drift gauge; read with `git diff --numstat bachuc-rebrand -- nix/module.nix` at completion.

## Verify

- `rg -n "paseo\.sh|hosted" nix/module.nix` -> no match.
- If `nix-instantiate` is installed: `nix-instantiate --parse nix/module.nix > /dev/null` -> exit 0. If it is not installed, record `UNVERIFIED (autonomous run)`.
- Anti-goal: `git diff --numstat bachuc-rebrand -- nix/module.nix` -> 30 or less.
