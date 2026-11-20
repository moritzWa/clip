#!/bin/sh
# Install a packaged build as a hidden, per-user launchd service.
# Never opens the panel or changes keyboard shortcuts/Accessibility grants.
set -eu
binary=${1:?usage: install.sh BINARY}
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
app="$HOME/Applications/Clip.app"
staging="$HOME/Applications/Clip-staging-$$.app"
label=com.moritz.clip
domain="gui/$(id -u)"
plist="$HOME/Library/LaunchAgents/$label.plist"
mkdir -p "$HOME/Applications" "$HOME/Library/LaunchAgents" "$HOME/Library/Logs/Clip"
umask 077
touch "$HOME/Library/Logs/Clip/stdout.log" "$HOME/Library/Logs/Clip/stderr.log"
chmod 600 "$HOME/Library/Logs/Clip/"*.log
sh "$root/scripts/package.sh" "$binary" "$staging"
if launchctl print "$domain/$label" >/dev/null 2>&1; then
    launchctl bootout "$domain/$label"
fi
if [ -e "$app" ]; then
    mv "$app" "$HOME/Applications/Clip-backup-$(date +%Y%m%d-%H%M%S).app"
fi
mv "$staging" "$app"
python3 - "$plist" "$app/Contents/MacOS/clip" "$HOME/Library/Logs/Clip" <<'PY'
import plistlib
import sys
path, binary, logs = sys.argv[1:]
with open(path, 'wb') as out:
    plistlib.dump({
        'Label': 'com.moritz.clip',
        'ProgramArguments': [binary],
        'RunAtLoad': True,
        'KeepAlive': True,
        'ThrottleInterval': 10,
        'ProcessType': 'Interactive',
        'LimitLoadToSessionType': 'Aqua',
        'StandardOutPath': logs + '/stdout.log',
        'StandardErrorPath': logs + '/stderr.log',
        'EnvironmentVariables': {'RUST_BACKTRACE': '1', 'HOME': __import__('os').environ['HOME']},
    }, out)
PY
plutil -lint "$plist"
launchctl bootstrap "$domain" "$plist"
launchctl print "$domain/$label"
