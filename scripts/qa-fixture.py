#!/usr/bin/env python3
"""Create synthetic QA data under an explicit HOME, never the real clipboard DB."""
import argparse
import pathlib
import sqlite3

parser = argparse.ArgumentParser()
parser.add_argument("home", type=pathlib.Path)
args = parser.parse_args()
home = args.home.resolve()
if home == pathlib.Path.home():
    raise SystemExit("Refusing to seed the real home directory")
root = home / "Library/Application Support/Clip"
root.mkdir(parents=True, exist_ok=True, mode=0o700)
db_path = root / "history.db"
if db_path.exists():
    raise SystemExit(f"Fixture already exists: {db_path}")
db = sqlite3.connect(db_path)
db.executescript("""
CREATE TABLE entries(id TEXT PRIMARY KEY, created_at TEXT NOT NULL, updated_at TEXT,
 title TEXT, categories TEXT, source_name TEXT, source_path TEXT,
 copy_count INTEGER, content_hash TEXT);
CREATE TABLE parts(id TEXT PRIMARY KEY, entry_id TEXT NOT NULL, kind TEXT NOT NULL,
 mime TEXT, text TEXT, file_path TEXT);
""")
doc = root / "preview-fixture.txt"
doc.write_text("Native document preview\n\nThis is a synthetic clipboard fixture.\nNo personal data.\n")
fixtures = [
    ("image", "Screenshot OCR fixture", "image", "", root / "ocr-fixture.png"),
    ("doc", doc.name, "file", "", doc),
    ("link", "https://example.com/native-clipboard", "link", "https://example.com/native-clipboard", None),
    ("text", "Whitespace stays intact", "text", "    let answer = 42;\n\n", None),
    ("batch-third", "Batch third copied", "text", "BATCH THIRD\n\n", None),
    ("batch-second", "Batch second copied", "text", "BATCH SECOND", None),
    ("batch-first", "Batch first copied", "text", "  BATCH FIRST\n", None),
    ("blank-title", " \t", "text", " \n\t   Actual visible first line\n    keep this indentation\n\n", None),
]
for index, (id_, title, kind, text, path) in enumerate(fixtures):
    stamp = f"2026-11-01T12:00:0{9-index}Z"
    db.execute("INSERT INTO entries VALUES (?,?,?,?,?,?,?,?,?)",
               (id_, stamp, stamp, title, '["' + kind + '"]', "QA Fixture", "", 1, id_))
    db.execute("INSERT INTO parts VALUES (?,?,?,?,?,?)",
               (id_ + ":p", id_, kind, "text/plain", text, str(path) if path else ""))
db.commit()
db.close()
print(root)
