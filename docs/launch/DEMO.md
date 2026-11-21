# Synthetic demo

The [36-second video](../media/clipglass-demo.mp4) records the real native app
inside a disposable macOS VM. It has no audio track. It shows:

- OCR search for `ORBIT-4827`, which appears only inside the generated image.
- An enlarged image preview, zoomed from Fit to 150%, then restored.
- A code snippet with its indentation intact.
- Three clips marked with X and pasted into a native editor in copy order.

The raw capture was completed before conversion, cropped to remove the VM menu
bar, and trimmed to 36 seconds. The result is H.264, 1024×720, 30 fps. No UI state
or clipboard result was composited into the recording. The full output decoded
without errors and a nine-frame contact sheet was visually reviewed.

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
