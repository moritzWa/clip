# Clip

A local macOS clipboard manager in Rust, using AppKit directly through `objc2`.
SQLite FTS5 indexes text and screenshot OCR. No webview, network service, expiry,
or application-level encryption.

## Controls

- Cmd+Shift+U: open/hide (physical W on DeutschADNW).
- On Moritz's Mac, Karabiner maps only left Cmd+U to that internal chord.
- Up/down: select a clip.
- Right: open a full-width image, document or selectable text preview.
- Left: return to the list. Escape returns from a preview, then hides the panel.
- In an image preview: pinch to zoom, two-finger scroll to pan, double-tap to
  toggle smart zoom. The −/Fit/+ controls work without a trackpad.
- Enter: copy the selected item and paste into the previous application.

Raycast's previous shortcut was removed manually. Automatic paste requires macOS
Accessibility permission for Clip.app. Without it, Enter still copies the item.

The borderless 750×475-point panel uses native macOS glass, a 26-point outer
radius, 38-point list rows, separate preview/information areas and glass footer
capsules. Type filters apply to the current result set (not the entire database).
Full previews preserve the query and selected row when returning to the list.
The expanded image and Quick Look document routes, Left/Escape return paths,
and retained query/selection were visually verified in the macOS VM.
Expanded previews use 90% of the available screen, capped at 1440×1000 points.
Image zoom ranges from Fit to 16× Fit, anchored under the pinch location. The
percentage is relative to Fit, not the image's original pixel resolution.

The VM-only `image-zoom-qa` example sends synthetic magnification events through
the production gesture selector and checks magnification, native panning, bounds,
Fit reset and smart zoom. Compile it with `cargo build --example image-zoom-qa`,
then run **inside a macOS VM only** with `CLIP_VM_TEST=1`. The app itself was also
visually checked at Fit and 225%, after horizontal scrolling and when returning
to the compact list. Physical trackpad hardware was not exercised by automation.

## Storage

Data lives under `~/Library/Application Support/Clip/`, never in this repository.
The source uses `history.db`, taking a consistent SQLite backup of the recovered
`import/history.db` on first use. Images are separate files. The initial import contains
1,100 entries and the OCR text Raycast had already generated.

The source now includes a bounded, single-worker Vision OCR queue, native Quick
Look document previews with autoplay disabled, transaction-protected writes,
whitespace-preserving text capture, and collision-resistant IDs. These changes
are included in the packaged build.

## Local installation

```sh
cargo build
sh scripts/install.sh "$PWD/target/debug/clip"
```

This installs `~/Applications/Clip.app` (stable ID `com.moritz.clip`, ad-hoc signed)
and a per-user launch agent. It launches hidden, starts at login, and restarts if
the process exits. Existing bundles are preserved as timestamped backups. It
does not change Accessibility permissions or keyboard settings. Enable Clip in
System Settings → Privacy & Security → Accessibility to allow automatic paste.

Logs are private files under `~/Library/Logs/Clip/`, including panic backtraces,
startup PID and Accessibility status, never clipboard contents. To stop it:

```sh
launchctl bootout "gui/$(id -u)/com.moritz.clip"
```

The old prototype exited without a retained stderr log or macOS crash report;
its original exit cause is unknown. The installed app is independent of the
agent session's shell lifecycle.

## Development

```sh
cargo test
cargo run -- search "example"
```

`cargo run` without arguments starts the native app. Do not use that as a host
GUI test. Test the UI in a macOS Space, with synthetic clipboard fixtures and
muted audio. Never copy real clipboard history into a shared test environment.

## VM validation (2026-11-20)

Tested an isolated debug app in `local:mac-for-desktop-testing`, with synthetic
data and audio muted. The host app was not relaunched or replaced.

- Vision recognized `BANANAS 4827` from a generated screenshot.
- Searching `bananas` found that screenshot and showed the OCR text.
- Quick Look rendered a text-file document preview.
- Right arrow focused the preview: Select All and Copy returned the OCR text.
- Left arrow returned to the search field.
- The global hotkey reopened the panel from a separate TextEdit instance.
- Enter pasted into TextEdit. Saving the result preserved the exact bytes
  `    let answer = 42;\n\n`, including indentation and trailing newlines.

The redesigned build was also tested with synthetic data: OCR search and
preview Select All/Copy, image type filtering, empty results, repeated hotkey
hide/show, and paste through both Enter and the footer button. Saved TextEdit
output matched the exact whitespace fixture. Native Quick Look was checked
separately. Packaging and hidden launch were tested through the installer;
launchd restarted the process after deliberate termination. No new host GUI
automation was used. Only VM test apps received automated Accessibility grants.

## Still required before replacing Raycast

- Verify rich clipboard formats and multi-file captures/paste.
- Move capture, large-file writes and search off the main thread.
- Add paging so browsing is not limited to the cached recent entries.
- Benchmark UI opening/search against large synthetic histories.
- Developer ID signing, notarization and in-app permissions UX.
- Database-wide type filtering and search highlighting. Time-range filters are backlogged.

## Measured storage performance

An isolated debug-build test with 100,000 synthetic entries (40 samples each)
measured these p95 times on this Mac:

| Operation | Initial implementation | Recency-ordered FTS index |
| --- | ---: | ---: |
| Recent 80 | 0.12 ms | 0.21 ms |
| Rare search | 18.15 ms | 0.67 ms |
| Common search | 176.47 ms | 7.08 ms |
| No matches | 16.03 ms | 0.11 ms |

Search is token-prefix based, not arbitrary substring matching. The unified
FTS row per entry includes its title, text and OCR. Its row order follows copy
recency, so searches can stop after the requested number of matches. OCR updates
retain the original order. A repeated copy moves the entry back to the top.

Run the benchmark with `cargo test benchmark_100k -- --ignored --nocapture`.
These are warm storage-query timings, not UI latency or a Raycast comparison.
Encryption alone does not establish Raycast's bottleneck.
