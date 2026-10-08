#!/usr/bin/env bash
# Builds a release from this checkout, installs it as "Bachuc.app", links ~/.local/bin/paseo to
# its CLI, and restarts the release daemon (~/.paseo on 127.0.0.1:6767) on it.
#
#   scripts/paseo-release.sh             build, install, start
#   scripts/paseo-release.sh --dry-run   print every change without making it
#   scripts/paseo-release.sh --force     install while agents are mid-turn
#   scripts/paseo-release.sh --skip-build  install the existing build in packages/desktop/release
#
# An install that fails after the daemon stopped puts the previous app back and starts it. From
# the install on, real runs append their output to ~/Library/Logs/Paseo/release-install.log; the
# terminal only mirrors that file, so closing the terminal cannot break a step or the rollback.
#
# Run it from Terminal.app, outside Paseo. The install stops the daemon that owns every Paseo agent
# and terminal, so a shell started by that daemon would die halfway through.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_HOME="$HOME/.paseo"
RELEASE_PORT=6767
RELEASE_LISTEN="127.0.0.1:$RELEASE_PORT"
# packages/desktop/src/main.ts gives the release home and port to this install path only.
APP="/Applications/Bachuc.app"
# The release app before the Bachuc rename. The first install replaces it and moves it to the Trash.
OLD_APP="/Applications/Paseo Fork.app"
BUILD_APP="$REPO_ROOT/packages/desktop/release/mac-arm64/Bachuc.app"
BUILD_BUNDLE_ID="io.github.hieppp.bachuc"
# Other builds are "Bachuc Dev". main.ts names the release app Bachuc, and the daemon finds its
# helper app by that name.
BUILD_NAME="Bachuc"
SIGN_IDENTITY="Paseo Fork Local"
# Agents call the paseo CLI through this link.
CLI_LINK="$HOME/.local/bin/paseo"
# The release loads hiep-plugins from this folder, as directory installs. The daemon compiles a
# plugin at load time and needs its type dependencies, which only a full checkout has.
PLUGINS_DIR="$REPO_ROOT/hiep-plugins/plugins"
OUTPUT_LOG="$HOME/Library/Logs/Paseo/release-install.log"
# Seconds the daemon gets to close its agents before the install gives up.
STOP_TIMEOUT=120
STAMP="$(date +%Y%m%d-%H%M%S)"

DRY_RUN=0
FORCE=0
SKIP_BUILD=0
PROBLEMS=""
STEP=""
STEP_NUMBER=0
# Set once the daemon is being stopped: from then on a failure can leave the home without one.
STOPPED=0
# The app that ran before: where it ran, and where it waits once the new one takes its place.
# Both are empty when no app ran.
PREVIOUS_PATH=""
PREVIOUS_APP=""
PREVIOUS_CLI_LINK=""
IN_ROLLBACK=0
MIRROR_PID=""

log() { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
rollback_failed() { printf 'error: rollback failed at step %s\n' "$STEP" >&2; }
die() {
  printf 'error: %s\n' "$*" >&2
  [ "$IN_ROLLBACK" = 0 ] || rollback_failed
  exit 1
}

# Every change goes through run, so --dry-run cannot touch anything. The command is printed in
# both modes, so the output log shows exactly what a real run did.
run() {
  printf '+'
  printf ' %q' "$@"
  printf '\n'
  [ "$DRY_RUN" = 0 ] || return 0
  "$@"
}

# move <from> <to>
# mv puts the source inside an existing directory instead of failing, so the destination must be
# free before the rename and must be the very same inode after it.
move() {
  local from="$1" to="$2" inode
  if [ "$DRY_RUN" = 1 ]; then
    run mv "$from" "$to"
    return 0
  fi
  if [ -e "$to" ] || [ -L "$to" ]; then
    die "refusing to move $from: $to already exists"
  fi
  inode="$(stat -f %i "$from")"
  run mv "$from" "$to"
  [ "$(stat -f %i "$to")" = "$inode" ] || die "moved $from, but $to is not it"
}

# Names the step in progress, so a failure says where it stopped.
step() {
  STEP_NUMBER=$((STEP_NUMBER + 1))
  STEP="[$STEP_NUMBER] $*"
  log "$STEP"
  # A dry run rehearses the rollback by failing at chosen step numbers, e.g. "7" or "7 9".
  if [ "$DRY_RUN" = 1 ]; then
    case " ${PASEO_RELEASE_FAIL_STEP:-} " in
      *" $STEP_NUMBER "*) die "simulated failure at step $STEP_NUMBER" ;;
    esac
  fi
}

problem() { PROBLEMS="${PROBLEMS}  - $*"$'\n'; }

report_problems() {
  [ -n "$PROBLEMS" ] || return 0
  printf 'Not ready to install:\n%s' "$PROBLEMS" >&2
  [ "$DRY_RUN" = 1 ] || exit 1
  warn "continuing because of --dry-run"
  PROBLEMS=""
}

# wait_for <seconds> <description> <command...>
wait_for() {
  local timeout="$1" what="$2" waited=0
  shift 2
  [ "$DRY_RUN" = 1 ] && return 0
  until "$@"; do
    [ "$waited" -lt "$timeout" ] || die "timed out after ${timeout}s waiting for $what"
    sleep 1
    waited=$((waited + 1))
  done
}

plist() {
  [ -f "$1" ] || return 0
  /usr/libexec/PlistBuddy -c "Print :$2" "$1" 2>/dev/null || true
}
app_version() { plist "$1/Contents/Info.plist" CFBundleShortVersionString; }
# The fork's builds are versioned X.Y.Z-hiep and shown as hiep-X.Y.Z, as the app shows them.
# Comparisons always use the raw version.
show_version() { printf '%s' "$1" | sed -E 's/^([0-9]+\.[0-9]+\.[0-9]+)-hiep$/hiep-\1/'; }
app_cli() { printf '%s' "$1/Contents/Resources/bin/paseo"; }
# -a: pgrep skips its own ancestors by default, which would hide the app from guard_outside_paseo.
# Matches the main executable whatever its name; the helper apps live under Contents/Frameworks.
app_pid() { pgrep -a -f "^$1/Contents/MacOS/[^/ ]+( |\$)" 2>/dev/null | head -1 || true; }
app_stopped() { [ -z "$(app_pid "$1")" ]; }

listener_pid() { lsof -nP -t -iTCP:"$RELEASE_PORT" -sTCP:LISTEN 2>/dev/null | head -1 || true; }
port_free() { [ -z "$(listener_pid)" ]; }
supervisor_pid() { jq -r '.pid // empty' "$RELEASE_HOME/paseo.pid" 2>/dev/null || true; }

# Runs a CLI against the release home whatever the calling shell exported.
release_cli() { env -u PASEO_LISTEN -u PASEO_HOST PASEO_HOME="$RELEASE_HOME" "$@"; }

# The installed release app: this install path, else the app before the rename, else none.
current_app() {
  if [ -d "$APP" ]; then
    printf '%s' "$APP"
  elif [ -d "$OLD_APP" ]; then
    printf '%s' "$OLD_APP"
  fi
}

# The CLI of the installed app; the build's before the first install.
installed_cli() {
  local current
  current="$(current_app)"
  if [ -n "$current" ] && [ -x "$(app_cli "$current")" ]; then
    app_cli "$current"
  else
    app_cli "$BUILD_APP"
  fi
}

guard_outside_paseo() {
  local reason="" owners pid owner
  if [ -n "${PASEO_AGENT_ID:-}" ]; then
    reason="PASEO_AGENT_ID is set, so this shell belongs to a Paseo agent"
  else
    owners="$(app_pid "$APP") $(app_pid "$OLD_APP") $(supervisor_pid) $(listener_pid)"
    pid=$$
    while [ -n "$pid" ] && [ "$pid" -gt 1 ]; do
      for owner in $owners; do
        [ "$pid" != "$owner" ] || reason="this shell runs inside the Paseo that owns the release home (ancestor PID $pid)"
      done
      pid="$(ps -o ppid= -p "$pid" 2>/dev/null | tr -d ' ' || true)"
    done
  fi
  [ -n "$reason" ] || return 0
  if [ "$DRY_RUN" = 1 ]; then
    warn "a real run refuses here: $reason"
    return 0
  fi
  die "$reason. Run this from Terminal.app, outside Paseo."
}

repo_plugin_ids() {
  local manifest
  for manifest in "$PLUGINS_DIR"/*/paseo-plugin.json; do
    [ -f "$manifest" ] && basename "$(dirname "$manifest")"
  done
  return 0
}

# Everything that can be checked before a build is spent.
preflight_home() {
  command -v jq >/dev/null 2>&1 || problem "jq is not on PATH"
  [ -f "$RELEASE_HOME/config.json" ] ||
    problem "no $RELEASE_HOME/config.json: the plugins cannot be registered"
  [ -n "$(repo_plugin_ids)" ] || problem "no plugins found in $PLUGINS_DIR"
  [ -w "$(dirname "$APP")" ] || problem "$(dirname "$APP") is not writable"
  [ ! -e "$CLI_LINK" ] || [ -L "$CLI_LINK" ] || problem "$CLI_LINK exists and is not a link"
  # The previous app goes to the Trash by rename, which is atomic only inside one volume.
  [ "$(stat -f %d "$HOME")" = "$(stat -f %d "$(dirname "$APP")")" ] ||
    problem "$HOME and $(dirname "$APP") are on different volumes"
}

preflight_build() {
  if [ ! -d "$BUILD_APP" ]; then
    problem "no build at $BUILD_APP"
    return 0
  fi
  [ "$(plist "$BUILD_APP/Contents/Info.plist" CFBundleIdentifier)" = "$BUILD_BUNDLE_ID" ] ||
    problem "build bundle ID is not $BUILD_BUNDLE_ID"
  [ "$(plist "$BUILD_APP/Contents/Info.plist" CFBundleName)" = "$BUILD_NAME" ] ||
    problem "build is not named $BUILD_NAME; build it with this script, not with --skip-build"
  [ -x "$(app_cli "$BUILD_APP")" ] || problem "build has no CLI at $(app_cli "$BUILD_APP")"
  # A reused build can be ad-hoc signed or cut off mid-signing; either loses the file access grants.
  case "$(codesign -dvv "$BUILD_APP" 2>&1 || true)" in
    *"Authority=$SIGN_IDENTITY"*) ;;
    *) problem "build is not signed by $SIGN_IDENTITY" ;;
  esac
  codesign --verify --deep --strict "$BUILD_APP" >/dev/null 2>&1 ||
    problem "build signature does not verify: codesign --verify --deep --strict \"$BUILD_APP\""
}

# Points every repo plugin at its folder in the release home's config. A plugin already listed
# keeps its enabled flag; a new one starts enabled. Entries for other plugins are left alone.
register_repo_plugins() {
  local config="$RELEASE_HOME/config.json" ids
  ids="$(repo_plugin_ids | jq -R . | jq -s .)"
  (umask 077 && jq --arg root "$PLUGINS_DIR" --argjson ids "$ids" '
    .pluginsEnabled = true
    | reduce $ids[] as $id (.;
        (.plugins[$id].enabled) as $enabled
        | .plugins[$id] = {
            source: "directory",
            path: ($root + "/" + $id),
            enabled: (if $enabled == null then true else $enabled end)
          })
  ' "$config" >"$config.tmp")
  mv "$config.tmp" "$config"
}

check_running_agents() {
  local cli="$1" agents running
  port_free && return 0
  if ! agents="$(release_cli "$cli" ls -a -g --json 2>/dev/null)"; then
    [ "$FORCE" = 1 ] || problem "cannot list agents on $RELEASE_LISTEN; pass --force to install anyway"
    return 0
  fi
  running="$(printf '%s' "$agents" | jq -r '.[] | select(.status == "running") | "\(.id[0:8]) \(.cwd // "")"')"
  [ -n "$running" ] || return 0
  if [ "$FORCE" = 1 ]; then
    warn "stopping the daemon under these running agents:"$'\n'"$running"
  else
    problem "agents are mid-turn; wait for them or pass --force:"$'\n'"$running"
  fi
}

stop_release() {
  local cli="$1" app pid
  for app in "$APP" "$OLD_APP"; do
    pid="$(app_pid "$app")"
    [ -n "$pid" ] || continue
    log "    quitting $app (PID $pid)"
    run kill -TERM "$pid"
    wait_for 60 "$app to quit" app_stopped "$app"
  done
  # The app stops its own daemon unless "keep running after quit" is on; this covers that case.
  # The exit code is not trusted: the free port below is the proof.
  [ ! -x "$cli" ] || run release_cli "$cli" daemon stop --timeout "$STOP_TIMEOUT" || true
  wait_for 30 "port $RELEASE_PORT to be free" port_free
  [ "$DRY_RUN" = 0 ] || return 0
  pid="$(supervisor_pid)"
  [ -n "$pid" ] || return 0
  if kill -0 "$pid" 2>/dev/null && ps -o command= -p "$pid" | grep -q Paseo; then
    die "daemon supervisor PID $pid is still running"
  fi
}

daemon_matches() {
  local status
  status="$(release_cli "$(app_cli "$1")" daemon status --json 2>/dev/null || true)"
  [ -n "$status" ] || return 1
  printf '%s' "$status" | jq -e --arg version "$(app_version "$1")" --arg listen "$RELEASE_LISTEN" \
    --arg home "$RELEASE_HOME" \
    '.localDaemon == "running" and .daemonVersion == $version and .listen == $listen and .home == $home' \
    >/dev/null 2>&1
}

# Every repo plugin is registered from the repo, and every enabled one is running.
plugins_ready() {
  local plugins
  plugins="$(release_cli "$(app_cli "$APP")" plugin ls --json 2>/dev/null || true)"
  [ -n "$plugins" ] || return 1
  printf '%s' "$plugins" | jq -e --arg root "$PLUGINS_DIR/" \
    --argjson expected "$(repo_plugin_ids | grep -c . || true)" \
    '[.[] | select(.path | startswith($root))]
     | length == $expected and all(.[]; (.enabled | not) or .status == "running")' \
    >/dev/null 2>&1
}

# start_and_verify <app>
start_and_verify() {
  run open -n "$1"
  wait_for 120 "daemon $(app_version "$1") on $RELEASE_LISTEN (see $RELEASE_HOME/daemon.log)" \
    daemon_matches "$1"
}

# Puts back the app that ran before and its CLI link, and starts it. Safe from any point after the
# daemon stopped.
rollback() {
  local cli app="${PREVIOUS_PATH:-$APP}"
  cli="$(app_cli "$APP")"
  [ -x "$cli" ] || cli="$(app_cli "$PREVIOUS_APP")"
  step "rollback: stop whatever the failed install started"
  stop_release "$cli"
  if [ -n "$PREVIOUS_PATH" ]; then
    step "rollback: put the previous app back"
    # $APP is the failed install unless the previous app still waits there.
    [ "$PREVIOUS_APP" = "$APP" ] || [ ! -e "$APP" ] ||
      move "$APP" "$HOME/.Trash/Bachuc failed $STAMP.app"
    [ "$PREVIOUS_APP" = "$PREVIOUS_PATH" ] || move "$PREVIOUS_APP" "$PREVIOUS_PATH"
    # Preflight refuses a CLI path that is not a link, so rm only removes the link this run made.
    if [ "$(readlink "$CLI_LINK" || true)" != "$PREVIOUS_CLI_LINK" ]; then
      if [ -n "$PREVIOUS_CLI_LINK" ]; then
        run ln -sfn "$PREVIOUS_CLI_LINK" "$CLI_LINK"
      else
        run rm -f "$CLI_LINK"
      fi
    fi
  fi
  [ -d "$app" ] || die "no previous app to go back to"
  step "rollback: start the previous app and wait for its daemon"
  start_and_verify "$app"
}

start_output_log() {
  local lines
  mkdir -p "$(dirname "$OUTPUT_LOG")"
  printf '=== %s %s ===\n' "$STAMP" "$*" >>"$OUTPUT_LOG"
  lines="$(wc -l <"$OUTPUT_LOG" | tr -d ' ')"
  # The terminal follows the file from this run's first line. The script itself writes only to
  # the file, so a closed terminal takes the mirror down and nothing else.
  tail -n "+$lines" -f "$OUTPUT_LOG" &
  MIRROR_PID=$!
  exec >>"$OUTPUT_LOG" 2>&1
}

stop_output_log() {
  [ -n "$MIRROR_PID" ] || return 0
  # Time for the mirror to print the last lines.
  sleep 0.5
  kill "$MIRROR_PID" 2>/dev/null || true
  MIRROR_PID=""
}

on_exit() {
  local code=$? rolled
  trap - EXIT
  # A second hangup must not stop the rollback.
  trap '' HUP
  if [ "$code" != 0 ]; then
    [ -z "$STEP" ] || printf 'error: failed at step %s\n' "$STEP" >&2
    if [ "$STOPPED" = 1 ]; then
      STOPPED=0
      log "Rolling back: the previous app takes the release home again."
      # A subshell, so a failing rollback step still reaches the report below. An EXIT trap does
      # not fire in a subshell started from this handler, so die and an ERR trap name the step.
      set +e
      (
        IN_ROLLBACK=1
        set -eE
        trap rollback_failed ERR
        rollback
      )
      rolled=$?
      set -e
      if [ "$rolled" = 0 ]; then
        log "Rolled back. The release home runs the previous app."
      else
        printf 'error: rollback did not finish, so the release home may have no daemon. No data was deleted. Open %s, or run again: scripts/paseo-release.sh\n' "${PREVIOUS_PATH:-$APP}" >&2
      fi
    fi
  fi
  stop_output_log
  exit "$code"
}

main() {
  local arg cli version previous_id staging="$APP.new"
  for arg in "$@"; do
    case "$arg" in
      --dry-run) DRY_RUN=1 ;;
      --force) FORCE=1 ;;
      --skip-build) SKIP_BUILD=1 ;;
      *) die "unknown argument: $arg (expected --dry-run, --force, --skip-build)" ;;
    esac
  done

  trap on_exit EXIT
  trap 'exit 129' HUP
  trap 'exit 130' INT TERM

  guard_outside_paseo

  step "check the release home and the running agents"
  preflight_home
  cli="$(installed_cli)"
  check_running_agents "$cli"
  report_problems

  cd "$REPO_ROOT"
  # The step stays numbered when skipped, so step numbers mean the same thing in every run.
  if [ "$SKIP_BUILD" = 1 ]; then
    step "reuse the build at $BUILD_APP"
    [ ! -d "$BUILD_APP" ] || log "    built $(stat -f %Sm "$BUILD_APP")"
  else
    step "build the release"
    # Signed with the self-signed SIGN_IDENTITY in the login keychain. Its designated requirement
    # is the certificate, not the cdhash, so macOS keeps file access grants across builds.
    # hardenedRuntime stays off: there is no Developer ID, and the app died at launch with it on.
    # timestamp=none: the secure timestamp is a network call per signed file (about 0.5s against
    # 0.05s without), across hundreds of files. Only notarization needs it.
    run npm run build:desktop -- --dir -c.productName="$BUILD_NAME" \
      -c.mac.hardenedRuntime=false -c.mac.notarize=false \
      -c.mac.identity="$SIGN_IDENTITY" -c.mac.timestamp=none
  fi
  [ "$DRY_RUN" = 1 ] || start_output_log "$@"

  step "check the build, and the running agents again"
  preflight_build
  check_running_agents "$cli"
  report_problems
  version="$(app_version "$BUILD_APP")"

  step "copy release $version to $staging"
  [ ! -e "$staging" ] || move "$staging" "$HOME/.Trash/Bachuc staging $STAMP.app"
  run ditto "$BUILD_APP" "$staging"

  PREVIOUS_PATH="$(current_app)"
  PREVIOUS_APP="$PREVIOUS_PATH"
  PREVIOUS_CLI_LINK="$(readlink "$CLI_LINK" || true)"
  previous_id="$(plist "$PREVIOUS_PATH/Contents/Info.plist" CFBundleIdentifier)"
  # From here the release home has no daemon until a start succeeds, so a failure rolls back.
  STOPPED=1
  step "stop the app and daemon on the release home"
  stop_release "$cli"
  step "register the plugins in $PLUGINS_DIR in the release home"
  run register_repo_plugins
  step "install $APP"
  if [ "$PREVIOUS_PATH" = "$APP" ]; then
    move "$APP" "$HOME/.Trash/Bachuc $STAMP.app"
    PREVIOUS_APP="$HOME/.Trash/Bachuc $STAMP.app"
  fi
  move "$staging" "$APP"
  step "start the app and wait for daemon $version"
  start_and_verify "$APP"
  step "wait for the plugins to run"
  wait_for 120 "plugins to run (check: paseo plugin ls, and $RELEASE_HOME/daemon.log)" plugins_ready
  step "link $CLI_LINK to the CLI of $APP"
  run mkdir -p "$(dirname "$CLI_LINK")"
  run ln -sfn "$(app_cli "$APP")" "$CLI_LINK"
  STOPPED=0
  # The new release runs, so a failure from here does not roll back. Run from a stale copy, the
  # old app would take the release home and restart the daemon on its older version.
  step "move the app before the rename to the Trash"
  if [ -e "$OLD_APP" ]; then
    move "$OLD_APP" "$HOME/.Trash/Paseo Fork $STAMP.app"
    [ "$PREVIOUS_PATH" != "$OLD_APP" ] || PREVIOUS_APP="$HOME/.Trash/Paseo Fork $STAMP.app"
  fi
  log "Release $(show_version "$version") is installed and runs on $RELEASE_LISTEN."
  [ -z "$PREVIOUS_APP" ] || log "The previous app is at $PREVIOUS_APP."
  # macOS ties privacy grants to the bundle ID, so a new bundle ID starts without them.
  if [ "$previous_id" != "$BUILD_BUNDLE_ID" ]; then
    log "The bundle ID is new: grant Bachuc microphone, screen recording, accessibility, and"
    log "automation again in System Settings > Privacy & Security."
  fi
}

# Sourcing the file loads the functions without running a command.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
