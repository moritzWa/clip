# Installation

Clipglass currently builds from source. No binary release, signed installer, or notarized download has been published.

## Requirements

- macOS 26 or later. The UI uses AppKit APIs introduced in macOS 26.
- An Apple Silicon Mac is the tested platform. Intel builds are unverified.
- Xcode Command Line Tools, including the macOS SDK and linker.
- Rust stable, installed through [rustup](https://rustup.rs/).
- Python 3, used to write the application and launch-agent property lists.

If the Command Line Tools are missing, run `xcode-select --install` and finish Apple's installer. Check your tools with:

```sh
xcode-select -p
rustc --version
cargo --version
python3 --version
```

## Build and install

During preparation, this GitHub repository is private and requires access. These commands build a development binary, not a release binary:

```sh
git clone https://github.com/moritzWa/clip.git clipglass
cd clipglass
cargo build --locked
sh scripts/install.sh "$PWD/target/debug/clip"
```

The installer:

- Packages the binary as `/Applications/Clipglass.app` with an ad-hoc signature.
- Requires write access to `/Applications`.
- Preserves replaced bundles and the older `~/Applications/Clip.app` as timestamped backups in `~/Applications`.
- Writes `~/Library/LaunchAgents/com.moritz.clip.plist`.
- Starts the app hidden, enables startup at login, and lets launchd restart it after an exit.
- Does not change keyboard mappings or grant Accessibility permission.

The visible app name is Clipglass. The bundle ID, `com.moritz.clip`, and storage directory remain unchanged. Installing does not delete clipboard history.

## Open history

The current internal shortcut is **Command+Shift+physical W**. That is **⌘⇧W on US** and **⌘⇧U on DeutschADNW**. Disable any conflicting binding in another app. There is no shortcut settings screen yet.

## Allow automatic paste

1. Open **System Settings → Privacy & Security → Accessibility**.
2. Add `/Applications/Clipglass.app` if it is not listed.
3. Enable **Clipglass**.
4. Restart the service if the running app has not picked up the grant:

```sh
launchctl kickstart -k "gui/$(id -u)/com.moritz.clip"
```

Without the grant, Enter copies text to the clipboard but cannot simulate a paste into another app. Mixed/media batches cannot be delivered automatically without the grant.

Ad-hoc signatures can change when rebuilding. macOS may require you to remove and re-add the app's Accessibility entry after an update.

## Troubleshooting

Check the background service:

```sh
launchctl print "gui/$(id -u)/com.moritz.clip"
```

Logs are in `~/Library/Logs/Clip/stdout.log` and `stderr.log`. Startup logs include the PID, paste keycode, and Accessibility status. Do not post clipboard databases or files in bug reports. Review any log before sharing it.

If the shortcut does nothing, check for a registration error in `stderr.log`, a competing shortcut in another app, or a stopped launch agent. Do not start a second copy alongside the service.

If Enter copies but does not paste, check Accessibility permission and whether the destination accepts the selected content type.

## Stop and restart

Stop the service for the current login session:

```sh
launchctl bootout "gui/$(id -u)/com.moritz.clip"
```

Start it again:

```sh
launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/com.moritz.clip.plist"
```

The launch-agent file remains after stopping, so the app can start again at the next login.

## Remove the app without deleting history

Stop the service first. Remove `~/Library/LaunchAgents/com.moritz.clip.plist`, then move `/Applications/Clipglass.app` to the Trash. Remove **Clipglass** from Accessibility if you no longer use it.

History stays in `~/Library/Application Support/Clip/`. Back up that folder before any separate decision to delete history. This guide does not delete it.
