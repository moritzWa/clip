#!/usr/bin/env python3
"""Bundle dependency notices without embedding local source paths or credentials."""
import json
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
output = Path(sys.argv[1])
output.mkdir(parents=True, exist_ok=True)
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--format-version", "1",
    "--filter-platform", "aarch64-apple-darwin",
], cwd=root))
ids = {node["id"] for node in metadata["resolve"]["nodes"]}
own = metadata["resolve"]["root"]
index = ["Clipglass dependency notices", "", "Clipglass itself is MIT licensed; see the adjacent LICENSE file.",
         "This inventory can include build dependencies that are not linked into the app.", ""]
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if package["id"] not in ids or package["id"] == own:
        continue
    source = Path(package["manifest_path"]).parent
    notices = [p for p in source.iterdir() if p.is_file() and
               p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))]
    if package.get("license_file"):
        explicit = source / package["license_file"]
        if explicit.is_file() and explicit not in notices:
            notices.append(explicit)
    if not notices and (package.get("repository") or "").startswith("https://github.com/madsmtm/objc2"):
        # Published workspace crates can omit their root notice.
        if "MIT" not in (package.get("license") or ""):
            raise SystemExit("Unexpected objc2 license declaration")
        notices = [root / "docs/licenses/objc2-MIT.txt"]
    if not notices:
        raise SystemExit("Missing license notice for " + package["name"])
    destination = output / (package["name"] + "-" + package["version"])
    destination.mkdir()
    for notice in notices:
        shutil.copyfile(notice, destination / notice.name)
    index.append(package["name"] + " " + package["version"] + " — " + (package.get("license") or "See notice"))

# Include notices for Rust's statically linked standard library as well.
sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
rust_docs = sysroot / "share/doc/rust"
if not (rust_docs / "COPYRIGHT-library.html").is_file():
    raise SystemExit("Install the rust-docs component to bundle Rust standard-library notices")
rust = output / "rust-standard-library"
rust.mkdir()
shutil.copyfile(rust_docs / "COPYRIGHT-library.html", rust / "COPYRIGHT-library.html")
shutil.copytree(rust_docs / "licenses", rust / "licenses")
index.append("Rust standard library — see rust-standard-library/COPYRIGHT-library.html")
(output / "INDEX.txt").write_text(chr(10).join(index) + chr(10))
print("Bundled dependency and Rust standard-library notices")
