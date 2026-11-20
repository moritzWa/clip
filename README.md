# Clip

A local macOS clipboard manager in Rust, using AppKit directly through `objc2`.
SQLite FTS5 indexes text and screenshot OCR. No webview, network service, expiry,
or application-level encryption.

## Prototype controls

- Cmd+Shift+U: open/hide (physical W on DeutschADNW).
- Up/down: select a clip.
- Right: focus the preview. Left: return to search.
- Enter: copy the selected item and paste into the previous application.
- Escape: hide.

Cmd+U remains assigned to Raycast. Automatic paste requires macOS Accessibility
permission. The current prototype does not install itself or start at login.

## Storage

Data lives under `~/Library/Application Support/Clip/`, never in this repository.
The source uses `history.db`, taking a consistent SQLite backup of the recovered
`import/history.db` on first use. The old running prototype still uses the import
directly. Images are separate files. The initial import contains
1,100 entries and the OCR text Raycast had already generated.

The source now includes a bounded, single-worker Vision OCR queue, native Quick
Look document previews with autoplay disabled, transaction-protected writes,
whitespace-preserving text capture, and collision-resistant IDs. These changes
are not yet loaded into the running prototype.

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

Only the VM test app received a sandbox Accessibility grant. The host's
permissions and hotkeys were not changed.

## Still required before replacing Raycast

- Verify rich clipboard formats and multi-file captures/paste.
- Move capture, large-file writes and search off the main thread.
- Add paging so browsing is not limited to the cached recent entries.
- Benchmark opening/search against large synthetic histories.
- Package a signed app, permissions UX and a login item.
- Add type filters and search highlighting. Time-range filters are backlogged.

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
