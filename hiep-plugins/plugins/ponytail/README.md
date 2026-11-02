# Ponytail

Keeps one [Ponytail](https://github.com/dietrichgebert/ponytail) mode per agent (`off`, `lite`,
`full`, `ultra`; default `full`) and adds the Ponytail rules to each Claude and Codex turn.

- The daemon hook sends the full ruleset when the mode differs from the last mode sent, and on
  every 10th turn. Other turns get a one-line reminder. Turning the mode `off` sends one stop line.
- A prompt that starts with `/ponytail [lite|full|ultra|off]` or `$ponytail ...` saves that mode
  and uses it for the turn.
- RPCs: `ponytail.mode.read` and `ponytail.mode.write`.
- `server/ponytail-hook.cjs` serves Claude sessions outside Paseo by running the native Ponytail
  scripts. It prints nothing inside a Paseo agent.

## Install

Install the plugin under the ID `ponytail`, keep the native `ponytail@ponytail` Claude plugin
disabled, then record the Ponytail folder and register the bridge in `~/.claude/settings.json`:

```sh
node scripts/install-hooks.mjs <ponytailRoot>
```

The folder holds `hooks/ponytail-instructions.js`. The daemon sends no context until
`$PASEO_HOME/plugin-data/ponytail/hook-runtime.json` exists.

## Remove

```sh
node scripts/install-hooks.mjs --remove
```

This deletes only the bridge entries from `~/.claude/settings.json`.
