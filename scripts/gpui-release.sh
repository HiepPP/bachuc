#!/usr/bin/env bash
# Builds the GPUI desktop client and installs it as "/Applications/Bachuc GPUI.app", the daily
# app on the release daemon (~/.bachuc on 127.0.0.1:6767). It also installs a login item that
# starts the release daemon, so the Electron app does not need to run.
#
#   scripts/gpui-release.sh              build, quit the app, install, add the login item, open
#   scripts/gpui-release.sh --skip-build install the bundle already in target/bundles
#
# The script quits the running GPUI app only after the build, so the app is down only while it
# installs. It never stops or restarts the daemon, so agents keep running and it can run inside
# Paseo. /Applications/Bachuc.app must stay installed: the daemon and the paseo CLI run on its
# runtime.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GPUI_DIR="$REPO_ROOT/packages/desktop-gpui"
NAME="Bachuc GPUI"
BUNDLE_ID="io.github.hieppp.bachuc-desktop"
BUILD_APP="$GPUI_DIR/target/bundles/$NAME.app"
APP="/Applications/$NAME.app"
SIGN_IDENTITY="Paseo Fork Local"
# The app opens on the host with this name in ~/Library/Application Support/BachucGpui/hosts.json.
HOST_NAME="release"
AGENT_LABEL="io.github.hieppp.bachuc-daemon"
AGENT_PLIST="$HOME/Library/LaunchAgents/$AGENT_LABEL.plist"
AGENT_LOG="$HOME/Library/Logs/Bachuc/daemon-login.log"
CLI="$HOME/.local/bin/paseo"

skip_build=0
for arg in "$@"; do
  case $arg in
    --skip-build) skip_build=1 ;;
    *) echo "usage: gpui-release.sh [--skip-build]" >&2; exit 2 ;;
  esac
done

if [ "$skip_build" -eq 0 ]; then
  "$GPUI_DIR/bundle.sh" bin bachuc-desktop "$NAME" "$BUNDLE_ID" >/dev/null
fi
[ -d "$BUILD_APP" ] || { echo "No bundle at $BUILD_APP" >&2; exit 1; }

pids="$(pgrep -f "$APP/Contents/MacOS/" || true)"
if [ -n "$pids" ]; then
  echo "Quitting $NAME (PID $(echo $pids))"
  kill -TERM $pids
  for _ in $(seq 60); do
    pgrep -f "$APP/Contents/MacOS/" >/dev/null || break
    sleep 1
  done
  if pgrep -f "$APP/Contents/MacOS/" >/dev/null; then
    echo "$NAME did not quit within 60 s; quit it, then run: $0 --skip-build" >&2
    exit 1
  fi
fi

plist="$BUILD_APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Delete :LSEnvironment" "$plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :LSEnvironment dict" \
  -c "Add :LSEnvironment:BACHUC_HOST string $HOST_NAME" "$plist"
# A signed bundle keeps its notification permission across installs.
codesign --force --sign "$SIGN_IDENTITY" "$BUILD_APP"

staged="/Applications/.$NAME.app.new"
[ ! -e "$staged" ] || mv "$staged" "$HOME/.Trash/$NAME.new.$(date +%Y%m%d-%H%M%S).app"
ditto "$BUILD_APP" "$staged"
[ ! -e "$APP" ] || mv "$APP" "$HOME/.Trash/$NAME.$(date +%Y%m%d-%H%M%S).app"
mv "$staged" "$APP"
echo "Installed $APP (host $HOST_NAME)"

# A login shell gives the daemon the user's PATH, as the Electron app does. `daemon start`
# reports "already running" and changes nothing when the daemon runs.
mkdir -p "$(dirname "$AGENT_PLIST")" "$(dirname "$AGENT_LOG")"
cat > "$AGENT_PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$AGENT_LABEL</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/zsh</string>
    <string>-l</string>
    <string>-i</string>
    <string>-c</string>
    <string>PASEO_HOME="\$HOME/.bachuc" exec "$CLI" daemon start</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <!-- The daemon outlives this job; launchd must not kill its process group. -->
  <key>AbandonProcessGroup</key>
  <true/>
  <key>StandardOutPath</key>
  <string>$AGENT_LOG</string>
  <key>StandardErrorPath</key>
  <string>$AGENT_LOG</string>
</dict>
</plist>
EOF
plutil -lint "$AGENT_PLIST" >/dev/null
if ! launchctl print "gui/$(id -u)/$AGENT_LABEL" >/dev/null 2>&1; then
  launchctl bootstrap "gui/$(id -u)" "$AGENT_PLIST"
fi
echo "Login item $AGENT_LABEL starts the release daemon at login (log: $AGENT_LOG)"

open "$APP"
echo "Opened $APP"
