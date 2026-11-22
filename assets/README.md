# Clipglass icon

`Clipglass.svg` is the approved Dual Glass artwork. `Clipglass.png` is the
transparent 1024px render used in the README. `Clipglass.icns` contains the native
macOS icon sizes and is copied into the app bundle during packaging.

To regenerate the PNG and ICNS on macOS, install Pillow in your Python environment
and install the SVG renderer:

```sh
npm install --prefix target-vm/icon-render --no-audit --no-fund @resvg/resvg-js
python3 scripts/build-icon.py
```

The renderer is a development-only dependency. Installing Clipglass does not
download icon dependencies or regenerate the artwork.
