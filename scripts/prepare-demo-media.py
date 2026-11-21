#!/usr/bin/env python3
"""Crop reviewed VM captures, stripping the VM menu bar. Requires Pillow."""
import pathlib
from PIL import Image

root = pathlib.Path(__file__).resolve().parent.parent
output = root / "docs/media"
output.mkdir(parents=True, exist_ok=True)
captures = {
    "history-clean.png": "clipglass-history.png",
    "ocr-search.png": "clipglass-ocr-search.png",
    "zoom-final.png": "clipglass-image-zoom.png",
    "code.png": "clipglass-code.png",
    "batch-final.png": "clipglass-batch.png",
    "pasted-final.png": "clipglass-pasted.png",
}
for source, name in captures.items():
    with Image.open(root / "target-vm" / source) as image:
        assert image.size == (1024, 768), image.size
        image.crop((0, 32, 1024, 752)).save(output / name)
print(f"Prepared {len(captures)} synthetic VM screenshots in {output}")
