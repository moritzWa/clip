# Installation

## Download the app (recommended)

Download the [signed, notarized Apple Silicon app](https://github.com/moritzWa/clip/releases/download/v0.1.0/Clipglass-0.1.0-macos-arm64.zip).
Unzip it, drag Clipglass.app to Applications, and open it. Requires macOS 26+ and
an M1-or-newer Mac. No developer tools are needed for this route.

The first launch opens history. After that, use **⌘⇧V**, **⌃⌘V** for images,
or the menu-bar clipboard icon. Shortcut conflicts are shown in Settings (⌘,).
Enable Accessibility for Clipglass in System Settings to paste automatically.
Without that grant, Enter copies the selected clip to the clipboard.

To start at login, add the app in System Settings → General → Login Items.
To update, quit Clipglass, replace the app with the new download, and reopen it.
This does not delete local clipboard history.

Release archives contain only the app, never clipboard history or fixtures.
Apple's notarization ticket is stapled to the bundle. SHA256SUMS.txt on each
release page lets you verify the archive with shasum -a 256 -c SHA256SUMS.txt.

## Source-build requirements

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

The GitHub repository is public. These commands build a development binary, not a release binary:

```sh
git clone https://github.com/moritzWa/clip.git clipglass
cd clipglass
cargo build --locked
rustup component add rust-docs
sh scripts/install.sh "$PWD/target/debug/clip"
```

The installer:

- Packages the binary as `/Applications/Clipglass.app`. It uses an available
  Developer ID certificate, or an ad-hoc signature if none is available.
- Requires write access to `/Applications`.
- Preserves replaced bundles and the older `~/Applications/Clip.app` as timestamped backups in `~/Applications`.
- Writes `~/Library/LaunchAgents/com.moritz.clip.plist`.
- Starts the app hidden, enables startup at login, and lets launchd restart it after an exit.
- Does not change keyboard mappings or grant Accessibility permission.

The visible app name is Clipglass. The bundle ID, `com.moritz.clip`, and storage directory remain unchanged. Installing does not delete clipboard history.

## Open history

Fresh installations use **⌘⇧V** for history and **⌃⌘V** for images. The image
shortcut toggles Images/All Types inside the panel. Existing installations retain
their old internal Command+Shift+physical W and Cmd+period bindings on migration.

Use the clipboard icon in the menu bar → **Settings…**, or **⌘,** while Clipglass
is focused, to record your own shortcuts. Changes take effect without restarting.
Escape cancels recording; **Restore Defaults** applies the fresh-install defaults.
Shortcuts are saved in `~/Library/Application Support/Clip/shortcuts.conf`.
**Show menu-bar icon** in Settings controls whether the menu-bar icon appears.
It takes effect immediately and persists across restarts. When hidden, open
history with its shortcut, then press **⌘,** to return to Settings.
Keyboard remappers still run first, so the recorder displays the combination
received by Clipglass rather than the physical chord before remapping.

## Allow automatic paste

1. Open **System Settings → Privacy & Security → Accessibility**.
2. Add `/Applications/Clipglass.app` if it is not listed.
3. Enable **Clipglass**.
4. Restart the service if the running app has not picked up the grant:

```sh
launchctl kickstart -k "gui/$(id -u)/com.moritz.clip"
```

Without the grant, Enter copies text to the clipboard but cannot simulate a paste into another app. Mixed/media batches cannot be delivered automatically without the grant.

Ad-hoc signatures change when rebuilding. macOS may require you to remove and
re-add the app's Accessibility entry after an update. A Developer ID signature
provides a stable identity across builds. `CLIP_SIGN_IDENTITY` can select a
specific signing identity; set it to `-` to force ad-hoc signing. Local signing
does not notarize the app or create a public release.

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
