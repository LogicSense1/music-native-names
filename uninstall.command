#!/bin/zsh
set -euo pipefail
app="$HOME/Applications/Music Native Names.app"
target="gui/$(id -u)/local.music-native-names.rust"
if [[ -x "$app/Contents/MacOS/music-native-names" ]]; then
  "$app/Contents/MacOS/music-native-names" --remove-login
else
  /bin/launchctl disable "$target"
  /bin/rm -f "$HOME/Library/LaunchAgents/local.music-native-names.rust.plist"
fi
if /bin/launchctl print "$target" >/dev/null 2>&1; then
  /bin/launchctl bootout "$target"
  for n in {1..90}; do
    /bin/launchctl print "$target" >/dev/null 2>&1 || break
    sleep 1
  done
  if /bin/launchctl print "$target" >/dev/null 2>&1; then
    print 'The helper is still restoring Music; the app has not been moved.'
    exit 1
  fi
fi
if [[ -d "$app" ]]; then
  mkdir -p "$HOME/.Trash"
  /bin/mv "$app" "$HOME/.Trash/Music Native Names-$(date +%Y%m%d%H%M%S).app"
fi
print 'Login startup disabled and app moved to Trash. Logs and cache were retained.'
