#!/usr/bin/env python3
"""Write the DMG window layout for the branded app into a staging folder.

    python3 brand/dmg.py <stage-dir> "<App Name>"

Reuses upstream's packaging/macos/dmg/generate.py (unchanged, so upstream merges stay clean) with
the fork's volume name, app name and background (brand/dmg/background.svg). Writes
<stage>/.background/background.tiff and <stage>/.DS_Store.

Needs resvg on PATH and: pip install 'pillow>=12' ds_store==1.3.3 mac_alias==2.2.3
Exit code 2 means a tool is missing; package.sh then builds a plain DMG without the layout.
"""

import os
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: dmg.py <stage-dir> <app name>")
    stage, name = sys.argv[1], sys.argv[2]
    if shutil.which("resvg") is None:
        print("dmg.py: resvg not found", file=sys.stderr)
        sys.exit(2)
    sys.path.insert(0, os.path.join(ROOT, "packaging", "macos", "dmg"))
    try:
        import generate  # upstream's generator
    except ImportError as e:
        print(f"dmg.py: {e}", file=sys.stderr)
        sys.exit(2)
    generate.VOLUME = name
    generate.ICONS = {f"{name}.app": (326, 205), "Applications": (574, 205)}
    os.makedirs(os.path.join(stage, ".background"), exist_ok=True)
    generate.write_tiff(os.path.join(HERE, "dmg", "background.svg"), os.path.join(stage, ".background", "background.tiff"))
    generate.write_ds_store(os.path.join(stage, ".DS_Store"))


if __name__ == "__main__":
    main()
