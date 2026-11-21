#!/usr/bin/env python3
"""Add editorial input callouts to the reviewed synthetic demo. Requires Pillow/ffmpeg.

Run video encoding through the machine's resource queue. These are hand-timed
callouts based on the recorded actions, not a live keyboard-event logger.
"""
import argparse
import pathlib
import subprocess
from PIL import Image, ImageDraw, ImageFont

parser = argparse.ArgumentParser()
parser.add_argument("source", type=pathlib.Path)
parser.add_argument("output", type=pathlib.Path)
args = parser.parse_args()
assert args.source.resolve() != args.output.resolve(), "Source and output must differ"
root = pathlib.Path(__file__).resolve().parent.parent
work = root / "target-vm/key-overlay"
work.mkdir(parents=True, exist_ok=True)
font_path = "/System/Library/Fonts/SFNS.ttf"
label_font = ImageFont.truetype(font_path, 19)
key_font = ImageFont.truetype(font_path, 24)

# Times correspond to the existing 36-second recording. Mouse-driven zoom is
# explicitly labelled as a click instead of inventing a '+' keyboard shortcut.
cues = [
    (0.0, 1.0, [], "Search"),
    (1.0, 4.5, ["ORBIT-4827"], "Type to search screenshot text"),
    (4.5, 8.0, ["→"], "Open large preview"),
    (8.0, 11.5, [], "Image preview"),
    (11.5, 15.0, ["+"], "Click to zoom"),
    (15.0, 18.5, [], "150% zoom"),
    (18.5, 19.5, ["←"], "Back to search"),
    (19.5, 23.0, ["⌘ A", "clipboard_history"], "Replace search"),
    (23.0, 25.5, [], "Code keeps its indentation"),
    (25.5, 26.5, ["⌘ A", "batch"], "Replace search"),
    (26.5, 30.5, ["X", "↓", "X", "↓", "X"], "Mark three clips"),
    (30.5, 33.0, ["Enter"], "Paste selected clips"),
    (33.0, 36.0, [], "Pasted in copy order, oldest first"),
]
playlist = []
for index, (start, end, keys, label) in enumerate(cues):
    image = Image.new("RGB", (1024, 80), "#223048")
    draw = ImageDraw.Draw(image)
    widths = [max(46, int(draw.textlength(key, font=key_font)) + 26) for key in keys]
    label_width = int(draw.textlength(label, font=label_font))
    total = sum(widths) + max(0, len(keys) - 1) * 10 + (22 if keys else 0) + label_width
    x = (1024 - total) // 2
    for key, width in zip(keys, widths):
        draw.rounded_rectangle((x, 17, x + width, 62), radius=9, fill="#F3F6FA")
        draw.text((x + width / 2, 40), key, anchor="mm", font=key_font, fill="#223048")
        x += width + 10
    if keys:
        x += 12
    draw.text((x, 40), label, anchor="lm", font=label_font, fill="#E4ECF7")
    path = work / f"cue-{index:02d}.png"
    image.save(path)
    playlist.extend([f"file '{path}'", f"duration {end-start:.3f}"])
playlist.append(f"file '{path}'")
manifest = work / "cues.ffconcat"
manifest.write_text("ffconcat version 1.0\n" + "\n".join(playlist) + "\n")
subprocess.run([
    "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
    "-i", str(args.source), "-f", "concat", "-safe", "0", "-i", str(manifest),
    "-filter_complex", "[0:v]fps=30,setsar=1[app];[1:v]fps=30,setsar=1[band];[app][band]vstack=inputs=2[v]",
    "-map", "[v]", "-t", "36", "-an", "-c:v", "libx264", "-preset", "medium",
    "-crf", "20", "-pix_fmt", "yuv420p", "-movflags", "+faststart", str(args.output),
], check=True)
