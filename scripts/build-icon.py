#!/usr/bin/env python3
"""Build transparent PNG/ICNS assets from the approved vector.

Requires Node, @resvg/resvg-js, Pillow and macOS iconutil. Packaging itself uses
the committed ICNS and needs none of these renderer dependencies.
"""
import os
from pathlib import Path
import subprocess
from PIL import Image

root = Path(__file__).resolve().parent.parent
assets = root / "assets"
work = root / "target-vm/icon-render"
iconset = work / "Clipglass.iconset"
iconset.mkdir(parents=True, exist_ok=True)
module = os.environ.get("CLIP_RESVG_MODULE", str(work / "node_modules/@resvg/resvg-js"))
subprocess.run(["node", "-e", """
const fs = require('fs');
const {Resvg} = require(process.argv[1]);
const svg = fs.readFileSync(process.argv[2]);
fs.writeFileSync(process.argv[3], new Resvg(svg).render().asPng());
""", module, str(assets / "Clipglass.svg"), str(assets / "Clipglass.png")], check=True)
with Image.open(assets / "Clipglass.png") as source:
    assert source.mode == "RGBA" and source.size == (1024, 1024)
    assert source.getpixel((0, 0))[3] == 0, "Icon corners must be transparent"
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            pixels = size * scale
            suffix = "@2x" if scale == 2 else ""
            source.resize((pixels, pixels), Image.Resampling.LANCZOS).save(
                iconset / f"icon_{size}x{size}{suffix}.png"
            )
subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(assets / "Clipglass.icns")], check=True)
print("Built transparent 1024px PNG and macOS ICNS")
