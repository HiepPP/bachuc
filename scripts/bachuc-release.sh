#!/usr/bin/env bash
# Builds and installs both release apps from this checkout, in one run:
#   1. scripts/paseo-release.sh: builds /Applications/Bachuc.app, whose runtime runs the daemon,
#      installs it, and restarts the release daemon (~/.bachuc on 127.0.0.1:6767) on it.
#   2. scripts/gpui-release.sh: builds and installs /Applications/Bachuc GPUI.app, the daily app.
#
#   scripts/bachuc-release.sh               build, install, and restart the daemon
#   scripts/bachuc-release.sh --skip-build  install the builds that already exist
#   scripts/bachuc-release.sh --force       restart the daemon while agents are mid-turn
#   scripts/bachuc-release.sh --dry-run     print the daemon steps only; skips the GPUI step
#
# gpui-release.sh alone never restarts the daemon, so a server change reaches release only
# through step 1. Run this from Terminal.app, outside Paseo: step 1 stops every running agent.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

dry_run=0
gpui_args=()
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry_run=1 ;;
    --skip-build) gpui_args+=("--skip-build") ;;
    --force) ;;
    *)
      printf 'error: unknown argument: %s (expected --dry-run, --force, --skip-build)\n' "$arg" >&2
      exit 1
      ;;
  esac
done

printf '== 1/2 daemon: scripts/paseo-release.sh %s\n' "$*"
"$REPO_ROOT/scripts/paseo-release.sh" "$@"

if [ "$dry_run" = 1 ]; then
  printf '== 2/2 GPUI: skipped, gpui-release.sh has no dry run\n'
  exit 0
fi

printf '== 2/2 GPUI: scripts/gpui-release.sh %s\n' "${gpui_args[*]:-}"
"$REPO_ROOT/scripts/gpui-release.sh" "${gpui_args[@]+"${gpui_args[@]}"}"

PASEO_HOME="$HOME/.bachuc" "$HOME/.local/bin/paseo" daemon status | grep -E 'startedAt|daemonVersion'
