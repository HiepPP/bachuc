# ADR-0080 Disabled native Ponytail plugin with a Paseo bridge over the enabled native plugin

Status: accepted
Date: 2026-11-02
Supersedes: -
Scope: `hiep-plugins/plugins/ponytail/**`, `~/.claude/settings.json`, `~/.claude/skills/ponytail*/**`

## Context

- Ponytail hooks keep the live mode in `~/.claude/.ponytail-active` and in a per-project file under `ponytail-modes/` (`hooks/ponytail-runtime.js`). Sessions in one repo share one mode; the last write wins.
- Paseo runs many agents in one repo, and each agent needs its own mode from its composer pill.
- Caveman has the same shared-flag problem, so `caveman@caveman` is disabled in `~/.claude/settings.json`, and a bridge hook plus the daemon `agent.prompt` hook add Caveman context instead.

## Decision

- Install the marketplace plugin `ponytail@ponytail` for its files, and keep it disabled in `enabledPlugins`.
- Copy its skills, except `band`, to `~/.claude/skills` so `/ponytail` and the review commands work.
- Paseo agents (Claude and Codex) get Ponytail rules from the daemon `agent.prompt` hook of the `ponytail` plugin, per agent.
- Claude sessions outside Paseo get the native Ponytail hooks through a bridge hook in `~/.claude/settings.json`. The bridge does nothing when `PASEO_AGENT_ID` is set.
- Hiep chose this on 2026-11-02 during `watchtower new`.

## Consequences

- The Ponytail status band and board do not run; they need the enabled native plugin.
- A Ponytail update needs the installer run again, because it records the plugin path with its version.
- Subagents of a Paseo agent do not get Ponytail rules.

## Options Considered

- Enable the native plugin: rejected. One agent's mode leaks into every Claude session in the same repo and overrides the pill.
- Skills only, with no hook outside Paseo: rejected. Plain Claude sessions would need a typed `/ponytail` each time.

## Revisit If

- Ponytail keys its mode by `session_id`, so concurrent sessions in one repo stop sharing one mode.
