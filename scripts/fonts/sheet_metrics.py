"""The sheet core's text metrics table, made from the drawing's own typefaces (docs/sheet/design.md §6).

A sheet's text (title blocks, legends, tables, grid labels) is laid out in
the Rust core, so a line breaks at the same word on the web and on the
desktop. The core measures with this table: per face of the drawing's
typefaces (ADR 0055: `DRAWING_FONTS` in apps/web/src/app/appearance.ts,
the WOFF2 files in apps/web/src/assets/fonts), the em size, the ascent,
descent and line gap the browser lays a line out with, the cap and x
heights, and the advance of every Latin and Turkish character the sheet
writes. The faces are the ones scripts/fonts/drawing_fonts.py builds for the
desktop (both subsets merged, variable fonts at 400, 500, 600 and 700), so
both platforms measure the letters they draw.

    python3 scripts/fonts/sheet_metrics.py           # writes crates/shared/sheet/data/font-metrics.json
    python3 scripts/fonts/sheet_metrics.py --check   # writes nothing; fails when the table is stale

A character a face does not have is -1 (the core takes the face's average
lowercase letter). Kerning is not in the table: phase 1 leaves it out
(design §6). Needs fontTools with brotli (WOFF2).
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

from drawing_fonts import WEIGHTS, faces, merged  # noqa: E402  (the same faces the desktop draws with)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates/shared/sheet/data/font-metrics.json"

# The CSS family → the DrawingFont id the sheet names it by (apps/web/src/app/appearance.ts).
IDS = {
    "Barlow": "barlow",
    "Arimo": "arimo",
    "Overpass": "overpass",
    "Quicksand": "quicksand",
    "Architects Daughter": "architects-daughter",
    "Courier Prime": "courier-prime",
    "IBM Plex Mono": "plex-mono",
}

# Basic Latin, Latin-1 Supplement and Latin Extended-A (the Turkish letters among them), and the
# punctuation, units and marks a sheet writes: quotes, dashes, the ellipsis, primes for DMS, the
# euro and lira signs, arrows, comparison signs and the missing-value marker's angle brackets.
EXTRA = [
    0x2013, 0x2014, 0x2018, 0x2019, 0x201A, 0x201C, 0x201D, 0x201E, 0x2022, 0x2026, 0x2030,
    0x2032, 0x2033, 0x2039, 0x203A, 0x20AC, 0x20BA, 0x2116, 0x2122, 0x2190, 0x2191, 0x2192,
    0x2193, 0x2212, 0x2248, 0x2260, 0x2264, 0x2265, 0x27E8, 0x27E9,
]
CHARS = [c for c in range(0x20, 0x7F)] + [c for c in range(0xA0, 0x180)] + EXTRA


def line_metrics(font):
    """Ascent, descent (positive) and line gap as Chrome lays a line out: OS/2 typo with USE_TYPO_METRICS, else hhea."""
    os2 = font["OS/2"]
    if os2.fsSelection & (1 << 7):
        return os2.sTypoAscender, -os2.sTypoDescender, os2.sTypoLineGap
    hhea = font["hhea"]
    return hhea.ascent, -hhea.descent, hhea.lineGap


def glyph_height(font, ch):
    """The top of a glyph's outline (for fonts whose OS/2 table has no cap or x height)."""
    glyph = font.getBestCmap().get(ord(ch))
    if glyph is None or "glyf" not in font:
        return 0
    g = font["glyf"][glyph]
    return getattr(g, "yMax", 0) or 0


def face_metrics(family, style, weight, font):
    cmap = font.getBestCmap()
    hmtx = font["hmtx"].metrics
    ascent, descent, gap = line_metrics(font)
    os2 = font["OS/2"]
    cap = getattr(os2, "sCapHeight", 0) or glyph_height(font, "H")
    x = getattr(os2, "sxHeight", 0) or glyph_height(font, "x")
    advances = [hmtx[cmap[c]][0] if c in cmap else -1 for c in CHARS]
    lower = [hmtx[cmap[c]][0] for c in range(ord("a"), ord("z") + 1) if c in cmap]
    return {
        "font": IDS[family],
        "weight": weight,
        "italic": style == "italic",
        "unitsPerEm": font["head"].unitsPerEm,
        "ascent": ascent,
        "descent": descent,
        "lineGap": gap,
        "capHeight": cap,
        "xHeight": x,
        "fallback": round(sum(lower) / len(lower)) if lower else font["head"].unitsPerEm // 2,
        "advances": advances,
    }


def build():
    out = []
    for family, style, weights, files in faces():
        wanted = [w for w in WEIGHTS if weights[0] <= w <= weights[-1]] if len(weights) == 2 else list(weights)
        for weight in wanted:
            if style == "italic" and weight != 400:
                continue
            out.append(face_metrics(family, style, weight, merged(family, style, weight, files)))
    order = list(IDS.values())
    out.sort(key=lambda f: (order.index(f["font"]), f["weight"], f["italic"]))
    return {
        "schema": "kentos.sheet.font-metrics/1",
        "generator": "scripts/fonts/sheet_metrics.py",
        "source": "apps/web/src/assets/fonts (the faces of scripts/fonts/drawing_fonts.py)",
        "chars": "".join(chr(c) for c in CHARS),
        "faces": out,
    }


def text(table):
    """One face per line: the file stays readable and its diffs small."""
    head = {k: v for k, v in table.items() if k != "faces"}
    lines = ["{"]
    for k, v in head.items():
        lines.append(f"  {json.dumps(k)}: {json.dumps(v, ensure_ascii=False)},")
    lines.append('  "faces": [')
    faces_text = [f"    {json.dumps(f, ensure_ascii=False, separators=(',', ':'))}" for f in table["faces"]]
    lines.append(",\n".join(faces_text))
    lines.append("  ]")
    lines.append("}")
    return "\n".join(lines) + "\n"


def main():
    table = build()
    want = text(table)
    if "--check" in sys.argv[1:]:
        have = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        if have != want:
            print(
                f"{OUT.relative_to(ROOT)} yazı tiplerine göre güncel değil: python3 scripts/fonts/sheet_metrics.py ile yeniden üretin.",
                file=sys.stderr,
            )
            return 1
        print(f"Pafta yazı ölçüleri güncel: {len(table['faces'])} yüz, {len(CHARS)} karakter.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(want, encoding="utf-8")
    print(f"{len(table['faces'])} yüz, {len(CHARS)} karakter yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
