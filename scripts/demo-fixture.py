#!/usr/bin/env python3
"""Generate a disposable demo HOME. Requires Pillow; never reads clipboard data."""
import argparse
import hashlib
import json
import pathlib
import sqlite3
from PIL import Image, ImageDraw, ImageFont

parser = argparse.ArgumentParser()
parser.add_argument("home", type=pathlib.Path)
args = parser.parse_args()
home = args.home.resolve()
if home == pathlib.Path.home():
    raise SystemExit("Refusing to seed the current user's home")
data = home / "Library/Application Support/Clip"
if (data / "history.db").exists():
    raise SystemExit("Refusing to overwrite an existing demo database")
files = data / "files"
files.mkdir(parents=True, exist_ok=True)
fonts = [
    pathlib.Path("/System/Library/Fonts/Supplemental/Arial.ttf"),
    pathlib.Path("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
]
font_path = next((p for p in fonts if p.exists()), None)
if font_path is None:
    raise SystemExit("Install an Arial or DejaVu Sans font for the synthetic image")


def font(size):
    return ImageFont.truetype(str(font_path), size)


image = Image.new("RGB", (1440, 1000), "#F4F6FA")
draw = ImageDraw.Draw(image)
draw.rectangle((0, 0, 260, 1000), fill="#24334B")
draw.text((32, 36), "ORBIT", font=font(42), fill="white")
for y, text in [(160, "Overview"), (228, "Launch plan"), (296, "Design"), (364, "Notes")]:
    if y == 228:
        draw.rectangle((20, y - 14, 240, y + 46), fill="#3E526F")
    draw.text((36, y), text, font=font(25), fill="#E5ECF5")
draw.text((320, 52), "Launch review", font=font(48), fill="#223048")
draw.text((322, 118), "Generated product data for the Clipglass demo", font=font(22), fill="#68768A")
draw.rectangle((320, 184, 1376, 472), fill="white")
draw.text((352, 214), "Release code", font=font(24), fill="#68768A")
draw.text((350, 266), "ORBIT-4827", font=font(74), fill="#273C61")
draw.text((354, 388), "Ready for review", font=font(26), fill="#34846A")
draw.rectangle((320, 514, 822, 846), fill="white")
draw.text((350, 548), "Design checks", font=font(30), fill="#223048")
for i, (text, color, length) in enumerate([
    ("Typography", "#729BC6", 360),
    ("Navigation", "#69AE9A", 310),
    ("Accessibility", "#B699D4", 330),
]):
    y = 615 + i * 67
    draw.text((352, y), text, font=font(19), fill="#65748A")
    draw.rectangle((352, y + 31, 352 + length, y + 42), fill=color)
draw.rectangle((858, 514, 1376, 846), fill="#E3EDF4")
draw.text((892, 548), "Next steps", font=font(30), fill="#223048")
for i, text in enumerate(["Review the screenshots", "Test the keyboard flow", "Approve the launch"]):
    draw.text((892, 627 + i * 58), text, font=font(23), fill="#3C5873")
draw.text((322, 922), "SYNTHETIC SCREENSHOT  /  NO PERSONAL DATA", font=font(19), fill="#7B8798")
picture = files / "design-review.png"
image.save(picture)
document = files / "release-notes.txt"
document.write_text("Clipglass demo notes\n\nAll content in this demo is generated.\nNo personal clipboard data is used.\n")
fixtures = [
    ("batch-one", "Batch 1: Plan the launch", "text", "01  Plan the launch", None),
    ("batch-two", "Batch 2: Review the demo", "text", "02  Review the demo", None),
    ("batch-three", "Batch 3: Publish after approval", "text", "03  Publish after approval", None),
    ("rust", "fn clipboard_history()", "text", "fn clipboard_history() {\n    let local = true;\n    let searchable = true;\n}\n", None),
    ("json", '{ "project": "clipglass" }', "text", '{\n  "project": "clipglass",\n  "storage": "local",\n  "demo": "synthetic"\n}\n', None),
    ("notes", "release-notes.txt", "file", "", document),
    ("link", "https://example.com/launch-notes", "link", "https://example.com/launch-notes", None),
    ("review-image", "Design review.png", "image", "", picture),
]
db = sqlite3.connect(data / "history.db")
db.executescript("""
CREATE TABLE entries(id TEXT PRIMARY KEY, created_at TEXT NOT NULL, updated_at TEXT,
 title TEXT, categories TEXT, source_name TEXT, source_path TEXT,
 copy_count INTEGER, content_hash TEXT);
CREATE TABLE parts(id TEXT PRIMARY KEY, entry_id TEXT NOT NULL, kind TEXT NOT NULL,
 mime TEXT, text TEXT, file_path TEXT);
""")
for index, (id_, title, kind, text, path) in enumerate(fixtures, 1):
    stamp = f"2026-11-21T09:00:{index:02d}Z"
    db.execute("INSERT INTO entries VALUES (?,?,?,?,?,?,?,?,?)", (
        "demo-" + id_, stamp, stamp, title, json.dumps([kind]), "Demo fixtures", "", 1,
        hashlib.sha256((id_ + text).encode()).hexdigest(),
    ))
    mime = "image/png" if kind == "image" else "text/plain"
    db.execute("INSERT INTO parts VALUES (?,?,?,?,?,?)", (
        "demo-" + id_ + ":part", "demo-" + id_, kind, mime, text, str(path) if path else "",
    ))
db.commit()
db.close()
(home / "SYNTHETIC-DEMO.json").write_text(json.dumps({
    "synthetic_only": True,
    "entries": len(fixtures),
    "generator": "scripts/demo-fixture.py",
    "reads_clipboard": False,
}, indent=2))
print(home)
