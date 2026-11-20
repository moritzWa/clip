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
The prototype uses `history.db`, falling back to the recovered
`import/history.db`. Images are separate files. The initial import contains
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

## Still required before replacing Raycast

- Validate OCR and Quick Look in a macOS Space.
- Verify rich clipboard formats and multi-file captures/paste.
- Move capture, large-file writes and search off the main thread.
- Add paging so browsing is not limited to the cached recent entries.
- Benchmark opening/search against large synthetic histories.
- Package a signed app, permissions UX and a login item.
- Make a separate immutable recovery archive and a live database.
- Add type filters and search highlighting. Time-range filters are backlogged.

No claim that this is faster than Raycast has been measured yet. Encryption
alone does not establish Raycast's bottleneck.
