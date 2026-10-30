#!/usr/bin/env python3
"""The KentOS Launcher cards' pictures (docs/adr/0800 §3): a card's picture
at twice its size, 720 × 360, cut to the card's 2:1 and kept as a 256-colour
PNG so the four stay well under 1.5 MB together.

    python3 scripts/launcher/pictures.py --fit mimari=plan.png harita=imar.png …
    python3 scripts/launcher/pictures.py --kentos target/release/kentos-cad

`--fit` takes the owner's own pictures (the bundled ones since 30 October
2026): each is cut about its middle to 2:1 and scaled. `--kentos` draws real
screenshots instead, without a window, with the desktop program's own
`snapshot` command from the repository's drawings and traces; it overwrites
the owner's pictures, so run it only to replace them. The pictures are
written into apps/launcher/assets/pictures (or --out), and their sizes are
said.
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

# Each screenshot: the snapshot's arguments (a drawing and its view, or a trace and
# its step) and the box cut from the 1440 × 900 window: left, top, right, bottom,
# twice as wide as high, inside the drawing area (0 … 1128 × 146 … 803).
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
}


def save(picture: Image.Image, path: Path) -> int:
    """A 256-colour PNG of the picture at the card's size; its size in bytes."""
    picture = picture.convert("RGB")
    if picture.size != SIZE:
        width, height = picture.size
        cut = min(height, width // 2)
        top, left = (height - cut) // 2, (width - 2 * cut) // 2
        picture = picture.crop((left, top, left + 2 * cut, top + cut)).resize(SIZE, Image.LANCZOS)
    picture.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.FLOYDSTEINBERG).save(
        path, optimize=True
    )
    size = path.stat().st_size
    print(f"{path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}: {size:,} bayt")
    return size


def screenshots(kentos: Path, out: Path) -> int | None:
    """KentOS's own screenshots; None when one went wrong (said)."""
    total = 0
    with tempfile.TemporaryDirectory(prefix="kentos-launcher-pictures-") as scratch:
        for name, (snapshot, box) in PICTURES.items():
            raw = Path(scratch) / f"{name}.png"
            subprocess.run(
                [str(kentos), "snapshot", str(raw), *snapshot],
                cwd=ROOT,
                check=True,
                stdout=subprocess.DEVNULL,
                timeout=300,
            )
            with Image.open(raw) as window:
                if window.size != (1440, 900):
                    print(f"{name}: pencere {window.size}, 1440 × 900 bekleniyordu", file=sys.stderr)
                    return None
                left, top, right, bottom = box
                if right - left != 2 * (bottom - top):
                    print(f"{name}: kesit iki eşit kare değil: {box}", file=sys.stderr)
                    return None
                total += save(window.crop(box), out / f"{name}.png")
    return total


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--fit", nargs="+", metavar="AD=RESİM", help="the owner's pictures, by card: mimari, harita, cbs, yol")
    parser.add_argument("--kentos", type=Path, help="the kentos-cad program (target/release/kentos-cad), for screenshots")
    parser.add_argument("--out", type=Path, default=OUT, help="where the pictures go")
    args = parser.parse_args()
    if bool(args.fit) == bool(args.kentos):
        print("Ya --fit ile resimleri ya da --kentos ile KentOS programını verin.", file=sys.stderr)
        return 2
    args.out.mkdir(parents=True, exist_ok=True)
    if args.kentos:
        if not args.kentos.is_file():
            print(f"{args.kentos}: KentOS programı yok; önce cargo build --release -p kentos-desktop", file=sys.stderr)
            return 1
        if screenshots(args.kentos, args.out) is None:
            return 1
    else:
        for item in args.fit:
            name, _, source = item.partition("=")
            if name not in PICTURES or not source:
                print(f"“{item}”: AD=RESİM yazın; adlar {', '.join(PICTURES)}.", file=sys.stderr)
                return 2
            with Image.open(source) as picture:
                save(picture, args.out / f"{name}.png")
    total = sum((args.out / f"{name}.png").stat().st_size for name in PICTURES if (args.out / f"{name}.png").is_file())
    print(f"toplam {total:,} bayt (sınır {BUDGET:,})")
    if total > BUDGET:
        print("Resimler çok büyük: kesitleri küçültün ya da renkleri azaltın.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
