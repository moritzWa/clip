# Synthetic demo

## Current screenshots

The history, OCR-search, code, batch-selection and image-preview stills were
recaptured on 2026-11-24 from the current native app in an isolated macOS 26 VM.
They show the compact rows, centered divider, tighter header and list-edge fades.
All 16 history entries and the screenshot fixture were generated for capture.
No personal history, documents or URLs were imported.

`CLIP_DEMO_BACKDROP_ONLY=1` starts `launch-demo-stage` with only its opaque
backdrop. The stills are crops of desktop pixels, preserving native glass and
shadows. `scripts/prepare-demo-media.py` applies the reviewed crop bounds.

## Earlier video

The [36-second video](../media/clipglass-demo.mp4) records the real native app
inside a disposable macOS VM. It has no audio track. It shows:

- OCR search for `ORBIT-4827`, which appears only inside the generated image.
- An enlarged image preview, zoomed from Fit to 150%.
- A code snippet with its indentation intact.
- Three clips marked with X and pasted into a native editor in copy order.

The raw capture was completed before conversion, cropped to remove the VM menu
bar, and trimmed to 36 seconds. The result is H.264, 1024×800, 30 fps. An 80-pixel
input-callout strip sits below the original 1024×720 recording without covering
the app. It shows typed queries, arrow keys, X selection, and Enter. The mouse
click on the zoom button is labelled as a click, not a keyboard shortcut.

These are hand-timed editorial callouts, not a live keyboard-event capture. No
app state or clipboard result was composited into the recording. Rebuild the
strip with `scripts/overlay-demo-keys.py`, using the original raw recording as
input. The full output decoded without errors and a contact sheet was visually
reviewed.

All text, URLs, documents, and images were generated for this demo. The only URL
fixture uses `example.com`. The demo used its own home directory and SQLite
database. It did not read or import personal clipboard history.

## Recreate fixtures

Install Pillow in a disposable Python environment, then:

```sh
python3 scripts/demo-fixture.py /path/to/disposable-demo-home
```

The script refuses the current user's actual home and refuses to overwrite an
existing database. Copy the generated home into the isolated macOS test machine
and remap the fixture file paths to their destination there. Run Clipglass with
that synthetic home. Never use a personal home for demo capture.

The VM-only `launch-demo-stage` example supplies an opaque backdrop and a native
paste destination. It requires `CLIP_VM_TEST=1`. The example has no capture or
network code. `scripts/prepare-demo-media.py` crops the reviewed VM screenshots;
it does not alter the application UI.
