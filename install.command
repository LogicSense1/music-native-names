#!/bin/zsh
set -euo pipefail
root="${0:A:h}"
source_app="$root/Music Native Names.app"
[[ -d "$source_app" ]] || source_app="$root/dist/Music Native Names.app"
app="$HOME/Applications/Music Native Names.app"
target="gui/$(id -u)/local.music-native-names.rust"
plist="$HOME/Library/LaunchAgents/local.music-native-names.rust.plist"
[[ -x "$source_app/Contents/MacOS/music-native-names" ]] || { print -u2 'App bundle missing. Build first, or extract the release ZIP completely.'; exit 1; }
[[ "$(uname -m)" == arm64 ]] || { print -u2 'Apple Silicon is required.'; exit 1; }
/usr/bin/xcrun --find lldb >/dev/null
/usr/bin/codesign --verify --strict "$source_app"
if ! /usr/bin/csrutil status | /usr/bin/grep -q 'status: disabled'; then
  print -u2 'Runtime installation requires an existing SIP-disabled configuration. No system settings were changed. See README.md.'
  exit 1
fi
if /bin/launchctl print "$target" >/dev/null 2>&1; then
  /bin/launchctl bootout "$target"
  for n in {1..90}; do
    /bin/launchctl print "$target" >/dev/null 2>&1 || break
    sleep 1
  done
  if /bin/launchctl print "$target" >/dev/null 2>&1; then
    print -u2 'The previous helper is still restoring Music. Retry when it exits.'
    exit 1
  fi
fi
if /usr/sbin/lsof -n -iTCP:18090 -sTCP:LISTEN >/dev/null 2>&1; then
  print -u2 'Port 18090 is already in use. Quit the existing helper before installing.'
  exit 1
fi
mkdir -p "$HOME/Applications"
if [[ -d "$app" ]]; then
  /bin/mv "$app" "$app.previous-$(date +%Y%m%d%H%M%S)"
fi
/usr/bin/ditto "$source_app" "$app"
/usr/bin/codesign --verify --strict "$app"
"$app/Contents/MacOS/music-native-names" --install-login
/bin/launchctl bootstrap "gui/$(id -u)" "$plist"
print 'Installed with login startup enabled. Look for the colored status symbol in the menu bar.'
