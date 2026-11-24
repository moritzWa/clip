#!/usr/bin/env python3
"""Crop reviewed desktop captures of the current native UI. Requires Pillow.

The VM display is 1024x768. Crops retain the app's native glass and shadow,
while excluding the VM menu bar, Dock and notifications. No UI is composited.
"""
import pathlib
from PIL import Image

root = pathlib.Path(__file__).resolve().parent.parent
output = root / "docs/media"
output.mkdir(parents=True, exist_ok=True)
captures = {
    "polished-history.png": ("clipglass-history.png", (104, 110, 920, 637)),
    "polished-ocr.png": ("clipglass-ocr-search.png", (104, 110, 920, 637)),
    "polished-zoom.png": ("clipglass-image-zoom.png", (24, 40, 1000, 704)),
    "polished-code.png": ("clipglass-code.png", (104, 110, 920, 637)),
    "polished-batch.png": ("clipglass-batch.png", (104, 110, 920, 637)),
}
for source, (name, bounds) in captures.items():
    with Image.open(root / "target-vm" / source) as image:
        assert image.size == (1024, 768), image.size
        image.crop(bounds).save(output / name)
print(f"Prepared {len(captures)} synthetic VM screenshots in {output}")
