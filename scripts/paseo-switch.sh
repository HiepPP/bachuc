#!/usr/bin/env bash
# Switches which app serves the stable home (~/.paseo on 127.0.0.1:6767).
#
#   scripts/paseo-switch.sh fork      install the current release as "Paseo Fork.app" and start it
#   scripts/paseo-switch.sh stock     go back to stock, /Applications/Paseo.app
#   scripts/paseo-switch.sh status    show which side is live and what a switch would use
#
# The release is this repo packaged; stock is the upstream app. "release" is still accepted as
# the older name of the stock subcommand.
#
# Options:
#   --dry-run  print every change without making it
#   --force    switch while agents are mid-turn
#
# While the fork is live the stock app is parked outside /Applications, so it cannot be opened
# on the fork data. Going back puts it in /Applications again.
#
# Each side keeps its own data. A switch renames the live ~/.paseo and userData into the slot of
# the side that was running, then renames the other side's slot into place. Nothing is deleted or
# merged, so each side comes back exactly as it was left. The first fork starts from a clone of
# the stock data.
#
# A switch that fails after the daemon stopped rolls back by itself to the side that ran before.
# Real runs append their output to ~/.paseo-switch/switch-output.log; the terminal only mirrors
# that file, so closing the terminal cannot break a step or the rollback.
#
# Run it from Terminal.app, outside Paseo. The switch stops the daemon that owns every Paseo agent
# and terminal, so a shell started by that daemon would die halfway through.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STABLE_HOME="$HOME/.paseo"
STABLE_PORT=6767
STABLE_LISTEN="127.0.0.1:$STABLE_PORT"
USER_DATA="$HOME/Library/Application Support/Paseo"
STOCK_APP="/Applications/Paseo.app"
# packages/desktop/src/main.ts keys the stable role on this bundle name.
FORK_APP="/Applications/Paseo Fork.app"
BUILD_APP="$REPO_ROOT/packages/desktop/release/mac-arm64/Paseo.app"
BUILD_BUNDLE_ID="sh.paseo.desktop.dev"
# The fork side loads hiep-plugins from this folder, as directory installs. The daemon compiles a
# plugin at load time and needs its type dependencies, which only a full checkout has.
PLUGINS_DIR="$REPO_ROOT/hiep-plugins/plugins"
CLI_LINKS=("$HOME/.local/bin/paseo" "/opt/homebrew/bin/paseo")
STATE_DIR="$HOME/.paseo-switch"
# Where the stock bundle waits while the fork is live. Opened then, it would restart the fork's
# daemon at the stock version and rewrite the fork data. The name does not end in .app because
# a moved bundle can still be reached through the Dock or a bookmark.
STOCK_PARKED="$STATE_DIR/apps/Paseo.app.parked"
SLOTS="$STATE_DIR/slots"
# The stock side was named "release" before, so stock data stashed then sits in this folder. It
# is read from here and never renamed; the next stash goes to $SLOTS/stock.
LEGACY_STOCK_SLOT="$SLOTS/release"
# Written into fork data only. Data without it belongs to stock, so the stock data is never
# modified by this script.
SIDE_LABEL=".paseo-switch-side"
OUTPUT_LOG="$STATE_DIR/switch-output.log"
# Seconds the daemon gets to close its agents before the switch gives up.
STOP_TIMEOUT=120
STAMP="$(date +%Y%m%d-%H%M%S)"

DRY_RUN=0
DRY_RUN_PARKED=0
FORCE=0
COMMAND=""
PROBLEMS=""
STEP=""
STEP_NUMBER=0
# Set once the daemon is being stopped: from then on a failure can leave the home without one.
STOPPED=0
# The side to go back to when a switch fails after that point; empty when there is none.
ROLLBACK_TO=""
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
  # A dry run rehearses the rollback by failing at chosen step numbers, e.g. "7" or "7 10".
  if [ "$DRY_RUN" = 1 ]; then
    case " ${PASEO_SWITCH_FAIL_STEP:-} " in
      *" $STEP_NUMBER "*) die "simulated failure at step $STEP_NUMBER" ;;
    esac
  fi
}

problem() { PROBLEMS="${PROBLEMS}  - $*"$'\n'; }

report_problems() {
  [ -n "$PROBLEMS" ] || return 0
  printf 'Not ready to switch:\n%s' "$PROBLEMS" >&2
  [ "$DRY_RUN" = 1 ] || exit 1
  warn "continuing because of --dry-run"
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
app_pid() { pgrep -a -f "^$1/Contents/MacOS/Paseo( |\$)" 2>/dev/null | head -1 || true; }
app_stopped() { [ -z "$(app_pid "$1")" ]; }
electron_version() {
  plist "$1/Contents/Frameworks/Electron Framework.framework/Versions/A/Resources/Info.plist" \
    CFBundleVersion
}

# The stock bundle, wherever it is right now.
stock_bundle() {
  if [ ! -d "$STOCK_APP" ] && [ -d "$STOCK_PARKED" ]; then
    printf '%s' "$STOCK_PARKED"
  else
    printf '%s' "$STOCK_APP"
  fi
}
stock_version() { app_version "$(stock_bundle)"; }

listener_pid() { lsof -nP -t -iTCP:"$STABLE_PORT" -sTCP:LISTEN 2>/dev/null | head -1 || true; }
port_free() { [ -z "$(listener_pid)" ]; }
supervisor_pid() { jq -r '.pid // empty' "$STABLE_HOME/paseo.pid" 2>/dev/null || true; }

# Runs a CLI against the stable home whatever the calling shell exported.
stable_cli() { env -u PASEO_LISTEN -u PASEO_HOST PASEO_HOME="$STABLE_HOME" "$@"; }

active_app() {
  if ! app_stopped "$FORK_APP"; then
    printf 'fork'
  elif ! app_stopped "$STOCK_APP" || ! app_stopped "$STOCK_PARKED"; then
    printf 'stock'
  else
    printf 'none'
  fi
}

cli_for() {
  if [ "$1" = fork ]; then app_cli "$FORK_APP"; else app_cli "$(stock_bundle)"; fi
}

live_path() {
  if [ "$1" = home ]; then printf '%s' "$STABLE_HOME"; else printf '%s' "$USER_DATA"; fi
}

# The side that owns a data folder: its label, "stock" when unlabelled, "none" when absent.
data_side() {
  if [ ! -d "$1" ]; then
    printf 'none'
  elif [ -f "$1/$SIDE_LABEL" ]; then
    cat "$1/$SIDE_LABEL"
  else
    printf 'stock'
  fi
}

# slot_path <side> <home|userData>
# Where a side's idle data sits. Resolved per folder, so an interrupted switch that left home and
# userData in different stock folders still finds both.
slot_path() {
  if [ "$1" = stock ] && [ ! -e "$SLOTS/stock/$2" ] && [ -e "$LEGACY_STOCK_SLOT/$2" ]; then
    printf '%s' "$LEGACY_STOCK_SLOT/$2"
  else
    printf '%s' "$SLOTS/$1/$2"
  fi
}

# stock_slot_twice <home|userData>
# Stock data in both stock folders: nothing says which copy is current, so no switch may pick one.
stock_slot_twice() { [ -e "$SLOTS/stock/$1" ] && [ -e "$LEGACY_STOCK_SLOT/$1" ]; }

# The folder shown for the stock slot: the old one while it still holds data.
stock_slot_dir() {
  if [ -e "$LEGACY_STOCK_SLOT/home" ] || [ -e "$LEGACY_STOCK_SLOT/userData" ]; then
    printf '%s' "$LEGACY_STOCK_SLOT"
  else
    printf '%s' "$SLOTS/stock"
  fi
}

label_fork() { printf 'fork\n' >"$1/$SIDE_LABEL"; }

guard_outside_paseo() {
  local reason="" owners pid owner
  if [ -n "${PASEO_AGENT_ID:-}" ]; then
    reason="PASEO_AGENT_ID is set, so this shell belongs to a Paseo agent"
  else
    owners="$(app_pid "$STOCK_APP") $(app_pid "$STOCK_PARKED") $(app_pid "$FORK_APP") $(supervisor_pid) $(listener_pid)"
    pid=$$
    while [ -n "$pid" ] && [ "$pid" -gt 1 ]; do
      for owner in $owners; do
        [ "$pid" != "$owner" ] || reason="this shell runs inside the stable Paseo (ancestor PID $pid)"
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

preflight_common() {
  command -v jq >/dev/null 2>&1 || problem "jq is not on PATH"
  if [ -d "$STOCK_APP" ] && [ -d "$STOCK_PARKED" ]; then
    problem "stock app exists twice: $STOCK_APP and $STOCK_PARKED"
  elif [ ! -d "$STOCK_APP" ] && [ ! -d "$STOCK_PARKED" ]; then
    problem "stock app missing: neither $STOCK_APP nor $STOCK_PARKED exists"
  fi
  [ "$(stat -f %d "$HOME")" = "$(stat -f %d "$(dirname "$STOCK_APP")")" ] ||
    problem "$HOME and $(dirname "$STOCK_APP") are on different volumes"
  # A rename is atomic only inside one volume.
  [ "$(stat -f %d "$HOME")" = "$(stat -f %d "$(dirname "$USER_DATA")")" ] ||
    problem "$HOME and $(dirname "$USER_DATA") are on different volumes"
}

# Each side's data must exist exactly once, live or in its slot, before anything is stopped.
preflight_data() {
  local target="$1" name live side other
  for name in home userData; do
    live="$(live_path "$name")"
    side="$(data_side "$live")"
    case "$side" in
      stock | fork | none) ;;
      *)
        problem "unknown side label '$side' in $live"
        continue
        ;;
    esac
    [ "$side" = none ] || [ ! -e "$(slot_path "$side" "$name")" ] ||
      problem "$side $name exists twice: $live and $(slot_path "$side" "$name")"
    ! stock_slot_twice "$name" ||
      problem "stock $name exists twice: $SLOTS/stock/$name and $LEGACY_STOCK_SLOT/$name"
    [ ! -e "$live.cloning" ] || warn "$live.cloning is left from an interrupted fork; it moves to $STATE_DIR/leftover"
    # The first fork clones the stock data, so the stock data is enough for either target.
    for other in "$target" stock; do
      if [ "$side" = "$other" ] || [ -d "$(slot_path "$other" "$name")" ]; then continue 2; fi
    done
    problem "no $name data for $target: $live is $side and $SLOTS holds none"
  done
}

preflight_build() {
  local resources="$BUILD_APP/Contents/Resources"
  if [ ! -d "$BUILD_APP" ]; then
    problem "no release at $BUILD_APP"
    return 0
  fi
  [ "$(plist "$BUILD_APP/Contents/Info.plist" CFBundleIdentifier)" = "$BUILD_BUNDLE_ID" ] ||
    problem "release bundle ID is not $BUILD_BUNDLE_ID"
  [ -x "$(app_cli "$BUILD_APP")" ] || problem "release has no CLI at $(app_cli "$BUILD_APP")"
  # The asar is an uncompressed archive, so the compiled main process is searchable.
  grep -q "Paseo Fork.app" "$resources/app.asar" 2>/dev/null ||
    problem "release predates the stable-fork role in main.ts; rebuild it"
  [ ! -d "$resources/plugins" ] ||
    problem "release still bundles plugins and would register them over the repo's; rebuild it"
  [ -n "$(repo_plugin_ids)" ] || problem "no plugins found in $PLUGINS_DIR"
  [ -w "$(dirname "$FORK_APP")" ] || problem "$(dirname "$FORK_APP") is not writable"
  [ "$(electron_version "$BUILD_APP")" = "$(electron_version "$(stock_bundle)")" ] ||
    warn "Electron differs: release $(electron_version "$BUILD_APP"), stock $(electron_version "$(stock_bundle)")."
}

repo_plugin_ids() {
  local manifest
  for manifest in "$PLUGINS_DIR"/*/paseo-plugin.json; do
    [ -f "$manifest" ] && basename "$(dirname "$manifest")"
  done
  return 0
}

# Points every repo plugin at its folder in the fork home's config. A plugin already listed
# keeps its enabled flag; a new one starts enabled. Entries for other plugins are left alone.
register_repo_plugins() {
  local config="$STABLE_HOME/config.json" ids
  [ "$(data_side "$STABLE_HOME")" = fork ] || die "$STABLE_HOME does not hold fork data; its plugin list is not rewritten"
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
  if ! agents="$(stable_cli "$cli" ls -a -g --json 2>/dev/null)"; then
    [ "$FORCE" = 1 ] || problem "cannot list agents on $STABLE_LISTEN; pass --force to switch anyway"
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

stop_stable() {
  local cli="$1" app pid
  for app in "$FORK_APP" "$STOCK_APP" "$STOCK_PARKED"; do
    pid="$(app_pid "$app")"
    [ -n "$pid" ] || continue
    log "    quitting $app (PID $pid)"
    run kill -TERM "$pid"
    wait_for 60 "$app to quit" app_stopped "$app"
  done
  # The app stops its own daemon unless "keep running after quit" is on; this covers that case.
  # The exit code is not trusted: the free port below is the proof.
  run stable_cli "$cli" daemon stop --timeout "$STOP_TIMEOUT" || true
  wait_for 30 "port $STABLE_PORT to be free" port_free
  [ "$DRY_RUN" = 0 ] || return 0
  # A lock that outlives its daemon would travel into the slot and could match an unrelated PID
  # by the time the data comes back.
  pid="$(supervisor_pid)"
  [ -n "$pid" ] || return 0
  if kill -0 "$pid" 2>/dev/null && ps -o command= -p "$pid" | grep -q Paseo; then
    die "daemon supervisor PID $pid is still running"
  fi
  run mkdir -p "$STATE_DIR/stale-locks"
  move "$STABLE_HOME/paseo.pid" "$STATE_DIR/stale-locks/$STAMP-paseo.pid"
}

# swap_dir <home|userData> <target side>
# Leaves the target side's data live. Safe to repeat from any interrupted state: the live folder
# is either absent or complete, and its label says whose it is.
swap_dir() {
  local name="$1" target="$2" live side stashed="" slot
  live="$(live_path "$name")"
  side="$(data_side "$live")"
  [ "$side" != "$target" ] || return 0
  ! stock_slot_twice "$name" ||
    die "stock $name exists twice: $SLOTS/stock/$name and $LEGACY_STOCK_SLOT/$name"
  if [ "$side" != none ]; then
    slot="$(slot_path "$side" "$name")"
    [ ! -e "$slot" ] || die "$side $name exists twice: $live and $slot"
    run mkdir -p "$(dirname "$slot")"
    move "$live" "$slot"
    stashed="$side"
  fi
  slot="$(slot_path "$target" "$name")"
  if [ -d "$slot" ]; then
    move "$slot" "$live"
    return 0
  fi
  [ "$target" = fork ] || die "no stock $name at $slot"
  [ -d "$(slot_path stock "$name")" ] || [ "$stashed" = stock ] || die "no stock $name to clone for the fork"
  # First fork. The clone is labelled before it is renamed into place, so an interrupted copy is
  # never mistaken for stock data. -c clones on APFS: no space is used until the sides diverge.
  if [ -e "$live.cloning" ]; then
    run mkdir -p "$STATE_DIR/leftover"
    move "$live.cloning" "$STATE_DIR/leftover/$STAMP-$name"
  fi
  run cp -Rcp "$(slot_path stock "$name")" "$live.cloning"
  run label_fork "$live.cloning"
  move "$live.cloning" "$live"
}

activate_data() {
  local name live
  swap_dir home "$1"
  swap_dir userData "$1"
  [ "$DRY_RUN" = 0 ] || return 0
  for name in home userData; do
    live="$(live_path "$name")"
    [ "$(data_side "$live")" = "$1" ] || die "$live holds $(data_side "$live") data after the swap, expected $1"
  done
}

# Renames only: the bundle is never copied or deleted. Both are safe to repeat.
park_stock() {
  [ -d "$STOCK_APP" ] || return 0
  [ ! -e "$STOCK_PARKED" ] || die "stock app exists twice: $STOCK_APP and $STOCK_PARKED"
  run mkdir -p "$(dirname "$STOCK_PARKED")"
  move "$STOCK_APP" "$STOCK_PARKED"
  DRY_RUN_PARKED="$DRY_RUN"
}

unpark_stock() {
  if [ -d "$STOCK_APP" ] && [ "$DRY_RUN_PARKED" = 0 ]; then return 0; fi
  [ -d "$STOCK_PARKED" ] || [ "$DRY_RUN_PARKED" = 1 ] || die "stock app missing: neither $STOCK_APP nor $STOCK_PARKED exists"
  move "$STOCK_PARKED" "$STOCK_APP"
}

# Repoints every paseo link that already points into one of the apps.
point_cli() {
  local link target
  for link in "${CLI_LINKS[@]}"; do
    target="$(readlink "$link" 2>/dev/null || true)"
    case "$target" in
      "$STOCK_APP"/* | "$STOCK_PARKED"/* | "$FORK_APP"/*)
        if [ -w "$(dirname "$link")" ]; then
          run ln -sfn "$(app_cli "$1")" "$link"
        else
          warn "$(dirname "$link") is not writable; $link still points at $target"
        fi
        ;;
      "") ;;
      *) warn "$link does not point into a Paseo app; left alone" ;;
    esac
  done
}

daemon_matches() {
  local status
  status="$(stable_cli "$(app_cli "$1")" daemon status --json 2>/dev/null || true)"
  [ -n "$status" ] || return 1
  printf '%s' "$status" | jq -e --arg version "$(app_version "$1")" --arg listen "$STABLE_LISTEN" \
    --arg home "$STABLE_HOME" \
    '.localDaemon == "running" and .daemonVersion == $version and .listen == $listen and .home == $home' \
    >/dev/null 2>&1
}

# Every repo plugin is registered from the repo, and every enabled one is running.
fork_plugins_ready() {
  local plugins
  plugins="$(stable_cli "$(app_cli "$FORK_APP")" plugin ls --json 2>/dev/null || true)"
  [ -n "$plugins" ] || return 1
  printf '%s' "$plugins" | jq -e --arg root "$PLUGINS_DIR/" \
    --argjson expected "$(repo_plugin_ids | grep -c . || true)" \
    '[.[] | select(.path | startswith($root))]
     | length == $expected and all(.[]; (.enabled | not) or .status == "running")' \
    >/dev/null 2>&1
}

start_and_verify() {
  local app="$1"
  run open -n "$app"
  wait_for 120 "daemon $(app_version "$app") on $STABLE_LISTEN (see $STABLE_HOME/daemon.log)" \
    daemon_matches "$app"
}

record() {
  mkdir -p "$STATE_DIR"
  printf '%s\t%s\n' "$STAMP" "$*" >>"$STATE_DIR/switch.log"
}

# Shared by the stock switch and by the rollback of a failed fork.
finish_stock() {
  step "put the fork data in its slot and bring the stock data back"
  activate_data stock
  # Only now: the stock app is launchable again once its own data is live.
  step "bring the stock app back to $STOCK_APP"
  unpark_stock
  step "point the paseo CLI links at stock"
  point_cli "$STOCK_APP"
  step "start stock and wait for daemon $(stock_version)"
  start_and_verify "$STOCK_APP"
}

# enter_fork and finish_fork are shared by fork and by the rollback of a failed switch to stock.
enter_fork() {
  step "park the stock app in $STOCK_PARKED"
  park_stock
  step "put the stock data in its slot and bring the fork data in"
  activate_data fork
}

# finish_fork <version>
finish_fork() {
  step "point the paseo CLI links at the fork"
  point_cli "$FORK_APP"
  step "start the fork app and wait for daemon $1"
  start_and_verify "$FORK_APP"
}

# rollback_to <side>
rollback_to() {
  local cli
  cli="$(app_cli "$(stock_bundle)")"
  [ ! -x "$(app_cli "$FORK_APP")" ] || cli="$(app_cli "$FORK_APP")"
  step "rollback: stop whatever the failed switch started"
  stop_stable "$cli"
  if [ "$1" = stock ]; then
    finish_stock
  else
    enter_fork
    finish_fork "$(app_version "$FORK_APP")"
  fi
  run record "rollback to $1 after a failed $COMMAND"
}

start_output_log() {
  local lines
  mkdir -p "$STATE_DIR"
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
  local code=$? target="$ROLLBACK_TO" rolled
  trap - EXIT
  # A second hangup must not stop the rollback.
  trap '' HUP
  if [ "$code" != 0 ]; then
    [ -z "$STEP" ] || printf 'error: failed at step %s\n' "$STEP" >&2
    if [ -n "$target" ]; then
      ROLLBACK_TO=""
      log "Rolling back: the $target side takes the stable home again."
      # A subshell, so a failing rollback step still reaches the report below. An EXIT trap does
      # not fire in a subshell started from this handler, so die and an ERR trap name the step.
      set +e
      (
        IN_ROLLBACK=1
        set -eE
        trap rollback_failed ERR
        rollback_to "$target"
      )
      rolled=$?
      set -e
      if [ "$rolled" = 0 ]; then
        log "Rolled back. Stable home runs the $target side on its own data."
      else
        printf 'error: rollback did not finish, so the stable home may have no daemon. No data was deleted. Run: scripts/paseo-switch.sh stock\n' >&2
      fi
    elif [ "$STOPPED" = 1 ]; then
      printf 'error: the stable home may have no daemon. No data was deleted. Run again: scripts/paseo-switch.sh %s\n' "$COMMAND" >&2
    fi
  fi
  stop_output_log
  exit "$code"
}

cmd_fork() {
  local active cli version staging="$FORK_APP.new"
  version="$(app_version "$BUILD_APP")"
  step "check the release and both sides' data"
  preflight_common
  preflight_build
  preflight_data fork
  active="$(active_app)"
  cli="$(cli_for "$active")"
  check_running_agents "$cli"
  report_problems
  guard_outside_paseo

  step "copy release $version to $staging"
  [ ! -e "$staging" ] || move "$staging" "$HOME/.Trash/Paseo Fork staging $STAMP.app"
  run ditto "$BUILD_APP" "$staging"

  # From here the stable home has no daemon until a start succeeds, so a failure rolls back.
  STOPPED=1
  ROLLBACK_TO=stock
  step "stop the stable app and daemon"
  stop_stable "$cli"
  enter_fork
  step "register the plugins in $PLUGINS_DIR in the fork home"
  run register_repo_plugins
  step "install $FORK_APP"
  [ ! -e "$FORK_APP" ] || move "$FORK_APP" "$HOME/.Trash/Paseo Fork $STAMP.app"
  move "$staging" "$FORK_APP"
  finish_fork "$version"
  step "wait for the plugins to run"
  wait_for 120 "plugins to run (check: paseo plugin ls, and $STABLE_HOME/daemon.log)" fork_plugins_ready
  ROLLBACK_TO=""
  run record "fork $version"
  log "Stable home now runs release $(show_version "$version") on the fork data."
}

cmd_stock() {
  local active cli previous
  step "check both sides' data"
  preflight_common
  preflight_data stock
  active="$(active_app)"
  cli="$(cli_for "$active")"
  previous="$(data_side "$STABLE_HOME")"
  if [ -z "$PROBLEMS" ] && [ "$active" = stock ] && [ -d "$STOCK_APP" ] &&
    [ "$previous" = stock ] && [ "$(data_side "$USER_DATA")" = stock ]; then
    log "Stable home already runs stock $(stock_version) on the stock data."
    return 0
  fi
  check_running_agents "$cli"
  report_problems
  guard_outside_paseo

  STOPPED=1
  # A failed switch to stock goes back to the fork only when the fork is what ran before.
  if [ "$previous" = fork ] && [ -d "$FORK_APP" ]; then ROLLBACK_TO=fork; fi
  step "stop the stable app and daemon"
  stop_stable "$cli"
  finish_stock
  ROLLBACK_TO=""
  run record "stock $(stock_version)"
  log "Stable home now runs stock $(stock_version) on the stock data."
}

slot_state() {
  local side="$1" name found=""
  for name in home userData; do
    [ ! -d "$(slot_path "$side" "$name")" ] || found="$found $name"
  done
  printf '%s' "${found:- empty}"
}

cmd_status() {
  local active daemon home_side link
  active="$(active_app)"
  home_side="$(data_side "$STABLE_HOME")"
  daemon="$(stable_cli "$(cli_for "$active")" daemon status --json 2>/dev/null || true)"
  log "live data     home: $home_side, userData: $(data_side "$USER_DATA")"
  log "active app    $active"
  [ "$active" = none ] || [ "$active" = "$home_side" ] ||
    log "MISMATCH      the $active app is running on $home_side data; run: scripts/paseo-switch.sh $home_side"
  log "daemon        $(printf '%s' "$daemon" | jq -r '"\(.localDaemon) \((.daemonVersion // "?") | sub("^(?<n>[0-9]+\\.[0-9]+\\.[0-9]+)-hiep$"; "hiep-\(.n)")) on \(.listen // "?") (PID \(.pid // "?"))"' 2>/dev/null || printf 'unknown')"
  for link in "${CLI_LINKS[@]}"; do
    log "cli link      $link -> $(readlink "$link" 2>/dev/null || printf 'not a symlink')"
  done
  log "stock slot   $(slot_state stock)   ($(stock_slot_dir))"
  log "fork slot    $(slot_state fork)   ($SLOTS/fork)"
  log "stock app     $(show_version "$(stock_version)") at $(stock_bundle)"
  [ "$home_side" != fork ] || [ ! -d "$STOCK_APP" ] ||
    log "WARNING       $STOCK_APP is installed while fork data is live; opening it would run stock on fork data"
  log "fork app      $([ -d "$FORK_APP" ] && show_version "$(app_version "$FORK_APP")" || printf 'not installed')"
  if [ -d "$BUILD_APP" ]; then
    log "release       $(show_version "$(app_version "$BUILD_APP")"), built $(date -r "$BUILD_APP/Contents/Resources/app.asar" '+%Y-%m-%d %H:%M')"
  else
    log "release       none at $BUILD_APP"
  fi
  log "fork plugins  $(repo_plugin_ids | grep -c . || true) in $PLUGINS_DIR"
  [ ! -f "$STATE_DIR/switch.log" ] || log "last switch   $(tail -1 "$STATE_DIR/switch.log")"
}

main() {
  local arg
  for arg in "$@"; do
    case "$arg" in
      --dry-run) DRY_RUN=1 ;;
      --force) FORCE=1 ;;
      fork | stock | status) COMMAND="$arg" ;;
      # The older name of the stock subcommand.
      release) COMMAND=stock ;;
      *) die "unknown argument: $arg (expected fork, stock, status, --dry-run, --force)" ;;
    esac
  done

  trap on_exit EXIT
  trap 'exit 129' HUP
  trap 'exit 130' INT TERM

  case "$COMMAND" in
    fork | stock)
      [ "$DRY_RUN" = 1 ] || start_output_log "$@"
      "cmd_$COMMAND"
      ;;
    status) cmd_status ;;
    *) die "usage: scripts/paseo-switch.sh <fork|stock|status> [--dry-run] [--force]" ;;
  esac
}

# Sourcing the file loads the functions without running a command.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
