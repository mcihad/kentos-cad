#!/usr/bin/env python3
"""The KentOS Launcher cards' pictures (docs/adr/0800 §3): real screenshots of
KentOS, drawn without a window by the desktop program's own `snapshot`
command from the repository's drawings and traces, cut to the drawing area
and scaled to a card's picture at twice its size (720 × 360 PNG).

    python3 scripts/launcher/pictures.py /path/to/kentos-cad [--out DIR]

The KentOS binary is given (a release build is quicker): the pictures are
redone after the desktop changes. They are written into
apps/launcher/assets/pictures (or --out), and their sizes are said; together
they must stay under 1.5 MB. The drawings are vector only: a raster's tiles
load in the background and a snapshot would catch them half drawn.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "apps/launcher/assets/pictures"
SIZE = (720, 360)
BUDGET = 1_500_000

# Each picture: the snapshot's arguments (a drawing and its view, or a trace and
# its step) and the box cut from the 1440 × 900 window: left, top, right, bottom,
# twice as wide as high, inside the drawing area (0 … 1128 × 146 … 803) or, for
# Rapor, over the bottom panel's chart.
PICTURES: dict[str, tuple[list[str], tuple[int, int, int, int]]] = {
    # A 3+1 house: walls, doors, rooms, furniture, the plot and its trees.
    "mimari": (
        ["samples/konut-3-1.kcad", "--tumu", "--merkez", "10.6,10.9", "--yakinlastir", "1.45"],
        (84, 234, 1044, 714),
    ),
    # Two blocks of parcels with their corner points, a street, contours and labels.
    "harita": (["fixtures/interaction/v1/label-engine.kcad", "--tumu"], (96, 316, 1000, 768)),
    # Thematic maps: bivariate colours, a heat map, displacement, charts, clusters, sizes.
    "cbs": (["fixtures/interaction/v1/renderers.kcad", "--tumu"], (326, 276, 1094, 660)),
    # A junction drawn with Plan yolu: roads, kerbs rounded, a median closed.
    "yol": (["--iz", "plan-road"], (118, 236, 1026, 690)),
    # A parcel layer's objects counted by block in a bar chart, under the map.
    "rapor": (["--iz", "charts", "--adim", "2"], (0, 272, 1128, 836)),
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("kentos", type=Path, help="the kentos-cad program (target/release/kentos-cad)")
    parser.add_argument("--out", type=Path, default=OUT, help="where the pictures go")
    args = parser.parse_args()
    if not args.kentos.is_file():
        print(f"{args.kentos}: KentOS programı yok; önce cargo build --release -p kentos-desktop", file=sys.stderr)
        return 1
    args.out.mkdir(parents=True, exist_ok=True)
    total = 0
    with tempfile.TemporaryDirectory(prefix="kentos-launcher-pictures-") as scratch:
        for name, (snapshot, box) in PICTURES.items():
            raw = Path(scratch) / f"{name}.png"
            subprocess.run(
                [str(args.kentos), "snapshot", str(raw), *snapshot],
                cwd=ROOT,
                check=True,
                stdout=subprocess.DEVNULL,
                timeout=300,
            )
            with Image.open(raw) as window:
                if window.size != (1440, 900):
                    print(f"{name}: pencere {window.size}, 1440 × 900 bekleniyordu", file=sys.stderr)
                    return 1
                left, top, right, bottom = box
                if right - left != 2 * (bottom - top):
                    print(f"{name}: kesit iki eşit kare değil: {box}", file=sys.stderr)
                    return 1
                picture = window.convert("RGB").crop(box).resize(SIZE, Image.LANCZOS)
            path = args.out / f"{name}.png"
            picture.save(path, optimize=True)
            size = path.stat().st_size
            total += size
            print(f"{path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}: {size:,} bayt")
    print(f"toplam {total:,} bayt (sınır {BUDGET:,})")
    if total > BUDGET:
        print("Resimler çok büyük: kesitleri küçültün ya da renkleri azaltın.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
