# Testing

## Unit checks

```sh
cargo test --locked
```

The suite covers whitespace preservation, readable list titles for old imports,
deduplication, transaction rollback, OCR indexing, safe FTS queries, copy-order
selection, and zoom geometry. The 100,000-entry benchmark is ignored by default.

## Native checks

Native UI checks ran in an isolated macOS 26 VM with synthetic history and audio
muted. No private clipboard database was moved into the VM.

- Vision recognized text in a generated image and search found it.
- Quick Look rendered a text document with autoplay disabled.
- Image previews enlarged, zoomed, panned, and returned to the compact list.
- Synthetic magnification events exercised the production pinch handler, zoom
  limits, Fit reset, and smart zoom. Physical trackpad hardware was not exercised.
- X marked and unmarked clips without intercepting X while editing a query.
- Selected text pasted oldest-copy first, retaining exact whitespace.
- A native receiver received text, file URL, and image payloads in copy order.
- Reopening the panel cancelled the remaining media queue.
- The thumbnail worker delivered a 64×40 thumbnail and reused the cache.
- Image files receive thumbnails too, including uppercase `.PNG` filenames.
  A synthetic image copied as a file showed its thumbnail in the native list.
  A store regression test covers PNG, JPEG, HEIC, and non-image file paths, and
  checks that file previews retain their native file-paste type.
- Old blank list titles displayed the first nonblank line. A paste receiver
  confirmed that the original spaces, newlines, and indentation remained intact.
- The packaged app launched hidden and launchd restarted it after termination.
- Closing a `blabla` search with the global hotkey and reopening showed an empty
  search and recent clips. Escape dismissal reset the search too. Right/Left
  preview navigation retained the query instead of clearing it.
- Chrome's native Copy Image command on a generated JPEG saved an image before
  any paste. Cmd+. opened Images and a second Cmd+. returned to All Types.
- The VM-only `clipboard-qa` example uses a unique native pasteboard: advertised
  PNG data was fulfilled after eight empty reads without changing changeCount,
  and captured without a paste. A JPEG-only clipboard normalized to a decodable
  TIFF. Retry state is bounded and partial text is not captured repeatedly.
- A database regression test puts an image file behind 220 text entries and
  checks that Images finds it both with and without a query. This covers type
  filtering before the result limit rather than filtering only a cached page.
- Compact-list geometry tests cover 30pt rows and a divider at 360pt in the
  750pt panel, retaining at least 11 visible rows and a preview wider than 350pt.
  Titles use 13pt text. A fresh VM visual check was unavailable because the
  desktop-testing VM was no longer registered.
- Cmd+, opened the native Settings window from the focused history panel.
  A recorded Cmd+Shift+K binding took effect immediately and appeared after a
  process restart. Duplicate history/image bindings were rejected without a
  settings-file change. Escape and closing the window cancelled recording and
  restored active shortcuts. Restore Defaults wrote Cmd+Shift+V / Ctrl+Cmd+V;
  both opened the expected history/image views. The menu-bar icon also opened
  Settings. Migration retained the earlier Cmd+Shift+physical W and Cmd+period
  bindings before any explicit change.

The `image-zoom-qa`, `paste-receiver-qa`, and `thumbnail-qa` examples are for VM-only
checks. They require `CLIP_VM_TEST=1`. The fixture seeder refuses to seed the
current user's actual home directory.

## Storage-query measurements

An isolated debug-build run used 100,000 synthetic entries and 40 samples per
operation. These are warm SQLite query timings, not UI latency or a comparison
against another clipboard manager.

| Operation | p95 |
| --- | ---: |
| Recent 80 | 0.21 ms |
| Rare search | 0.67 ms |
| Common search | 7.08 ms |
| No matches | 0.11 ms |

```sh
cargo test benchmark_100k -- --ignored --nocapture
```

Hardware, filesystem state, query contents, and cold caches affect results.
