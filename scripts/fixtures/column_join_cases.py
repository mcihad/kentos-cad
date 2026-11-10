#!/usr/bin/env python3
"""A column in the walls it stands in (docs/adr/3041 §2), from the ADR's rules, in exact fractions, without KentOS code.

Writes fixtures/walls/v1/column-joins.json; `--check` compares and writes nothing. The shared core's test
(`kentos-geometry-core`, tests/all/column_joins.rs) reads it and must give the same lines, caps, band areas and looks.

The walls here are straight, drawn eastwards and centred on their paths (y = c); their layers are given from the left
face, the +y side. The columns are rectangles square to the axes. The rules, as ADR 3041 §2 gives them:

- A wall's lines are its boundaries: the left face c + w/2, each layer's thickness further right, the right face
  c − w/2. Its caps are the segments across its ends, its bands the rectangles between consecutive lines.
- A column on the wall's level (both on none, or the same level) cuts it. A line keeps its parts whose points are not
  strictly inside the column; a point on the column's side is outside. A cap likewise. A band loses the column's
  rectangle.
- A column is one section with a band of its material whose rectangle shares an area with it. Materials are compared
  folded: trimmed, Turkish letters plain, lower case, so "BETONARME" is "Betonarme". A column naming none is
  Betonarme. It takes the first such band's hatch pattern and its wall's fill. Its outline is drawn except where it
  lies strictly inside such a band.
"""

import argparse
import json
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/walls/v1/column-joins.json"

FOLD = str.maketrans({"Ç": "c", "ç": "c", "Ğ": "g", "ğ": "g", "I": "i", "ı": "i", "İ": "i", "Ö": "o", "ö": "o",
                      "Ş": "s", "ş": "s", "Ü": "u", "ü": "u", "Â": "a", "â": "a", "Î": "i", "î": "i", "Û": "u",
                      "û": "u"})


def fold(name):
    return name.strip().translate(FOLD).lower()


def num(f):
    return float(f)


def layer(material=None, thickness=None, pattern=None, color=None, detail="standard"):
    c = {"detail": detail}
    if material is not None:
        c["material"] = material
    if thickness is not None:
        c["thickness"] = num(thickness)
        c["_t"] = F(thickness)
    if pattern is not None:
        hatch = {"pattern": pattern, "scale": 1.0, "angle": 0.0}
        if color is not None:
            hatch["color"] = color
        c["hatch"] = hatch
    return c


def wall(x0, x1, c, width, layers, fill=None, level=None):
    w = {"pts": [[num(x0), num(c)], [num(x1), num(c)]], "width": num(width), "justify": "center", "components": layers,
         "_exact": (F(x0), F(x1), F(c), F(width))}
    if fill is not None:
        w["fill"] = fill
    if level is not None:
        w["arch"] = {"level": level}
    return w


def column(x, y, width, depth=None, material=None, level=None):
    col = {"at": [num(x), num(y)], "width": num(width), "_exact": (F(x), F(y), F(width), F(depth if depth is not None else width))}
    if depth is not None:
        col["depth"] = num(depth)
    if material is not None:
        col["material"] = material
    if level is not None:
        col["arch"] = {"level": level}
    return col


def boundaries(w):
    """The wall's lines' heights, from its left face, and its layers' heights between them."""
    _, _, c, width = w["_exact"]
    top = c + width / 2
    given = [l["_t"] if "_t" in l else None for l in w["components"]]
    rest = width - sum(t for t in given if t is not None)
    ys = [top]
    for t in given:
        ys.append(ys[-1] - (t if t is not None else rest))
    if not w["components"]:
        ys.append(top - width)
    return ys


def rect(col):
    x, y, a, b = col["_exact"]
    return (x - a / 2, x + a / 2, y - b / 2, y + b / 2)


def same_level(w, col):
    return (w.get("arch") or {}).get("level") == (col.get("arch") or {}).get("level")


def cut_interval(lo, hi, cut_lo, cut_hi):
    """[lo, hi] less the open interval (cut_lo, cut_hi): its parts longer than nothing."""
    out = []
    if cut_hi <= lo or cut_lo >= hi:
        return [(lo, hi)]
    if cut_lo > lo:
        out.append((lo, cut_lo))
    if cut_hi < hi:
        out.append((cut_hi, hi))
    return out


def expect_wall(w, cols):
    x0, x1, _, _ = w["_exact"]
    ys = boundaries(w)
    cutters = [rect(c) for c in cols if same_level(w, c)]
    lines = []
    for y in ys:
        pieces = [(x0, x1)]
        for (cx0, cx1, cy0, cy1) in cutters:
            if cy0 < y < cy1:
                pieces = [p for (a, b) in pieces for p in cut_interval(a, b, cx0, cx1)]
        lines.append({"y": num(y), "pieces": [[num(a), num(b)] for a, b in pieces]})
    caps = []
    for x in (x0, x1):
        pieces = [(ys[-1], ys[0])]
        for (cx0, cx1, cy0, cy1) in cutters:
            if cx0 < x < cx1:
                pieces = [p for (a, b) in pieces for p in cut_interval(a, b, cy0, cy1)]
        caps += [[num(x), num(a), num(b)] for a, b in pieces]
    bands = []
    if w["components"]:
        for k in range(len(w["components"])):
            hi, lo = ys[k], ys[k + 1]
            area = (x1 - x0) * (hi - lo)
            # The columns' rectangles do not overlap one another here: each takes its own share.
            for (cx0, cx1, cy0, cy1) in cutters:
                ox = max(F(0), min(x1, cx1) - max(x0, cx0))
                oy = max(F(0), min(hi, cy1) - max(lo, cy0))
                area -= ox * oy
            bands.append(num(area))
    return {"lines": lines, "caps": caps, "bands": bands}


def expect_column(col, walls):
    cx0, cx1, cy0, cy1 = rect(col)
    mine = fold(col.get("material", "Betonarme"))
    look = None
    regions = []
    for w in walls:
        if not same_level(w, col):
            continue
        x0, x1, _, _ = w["_exact"]
        ys = boundaries(w)
        for k, l in enumerate(w["components"]):
            if "material" not in l or fold(l["material"]) != mine:
                continue
            hi, lo = ys[k], ys[k + 1]
            if min(x1, cx1) - max(x0, cx0) > 0 and min(hi, cy1) - max(lo, cy0) > 0:
                regions.append((x0, x1, lo, hi))
                if look is None:
                    look = ((l.get("hatch") or {}).get("pattern"), w.get("fill"))
    if look is None:
        return {"joined": False, "outline": num(2 * (cx1 - cx0) + 2 * (cy1 - cy0))}
    hidden = F(0)
    # Its sides: the horizontal ones at cy0 and cy1 from cx0 to cx1, the vertical at cx0 and cx1 from cy0 to cy1.
    for y in (cy0, cy1):
        parts = []
        for (x0, x1, lo, hi) in regions:
            if lo < y < hi:
                a, b = max(cx0, x0), min(cx1, x1)
                if b > a:
                    parts.append((a, b))
        hidden += union_length(parts)
    for x in (cx0, cx1):
        parts = []
        for (x0, x1, lo, hi) in regions:
            if x0 < x < x1:
                a, b = max(cy0, lo), min(cy1, hi)
                if b > a:
                    parts.append((a, b))
        hidden += union_length(parts)
    out = {"joined": True, "outline": num(2 * (cx1 - cx0) + 2 * (cy1 - cy0) - hidden)}
    if look[0] is not None:
        out["pattern"] = look[0]
    if look[1] is not None:
        out["fill"] = look[1]
    return out


def union_length(parts):
    total, end = F(0), None
    for a, b in sorted(parts):
        if end is None or a > end:
            total += b - a
            end = b
        elif b > end:
            total += b - end
            end = b
    return total


def strip(o):
    if isinstance(o, dict):
        return {k: strip(v) for k, v in o.items() if not k.startswith("_")}
    if isinstance(o, list):
        return [strip(v) for v in o]
    return o


def cases():
    brick = [layer("Tuğla", pattern="ANSI31")]
    plastered = [layer("Sıva", F("0.02"), detail="detailed"), layer("Tuğla", pattern="ANSI31"),
                 layer("Sıva", F("0.02"), detail="detailed")]
    concrete = [layer("Betonarme", pattern="SOLID", color="#A6A6A6")]
    shout = [layer("BETONARME", pattern="SOLID", color="#A6A6A6")]
    w20 = lambda layers=brick, **kw: wall(0, 4, 0, F("0.2"), layers, fill="#5A5A5A", **kw)
    w25 = lambda layers=concrete: wall(0, 4, 0, F("0.25"), layers, fill="#3C3C3C")
    col = lambda x, y, **kw: column(x, y, F("0.4"), **kw)
    out = [
        ("Duvar kolonun içinde biter: yüzleri kolonun yüzünde durur, oradaki ucu gider", [w20()], [col(4, 0)]),
        ("Kolon duvarın ortasında: duvar ikiye bölünür", [w20()], [col(2, 0)]),
        ("Kolonun kenarı duvarın sağ yüzünde: o yüz kalır, öbürü kesilir", [w20()], [col(2, F("0.1"))]),
        ("Kolon duvarın dışında: duvar değişmez", [w20()], [col(2, 1)]),
        ("Kolon duvara yalnız değer: duvar değişmez", [w20()], [col(2, F("0.3"))]),
        ("Başka kattaki kolon duvarı kesmez", [w20(level="zemin")], [col(4, 0, level="k1")]),
        ("Aynı kattaki kolon keser", [w20(level="zemin")], [col(4, 0, level="zemin")]),
        ("Sıvalı duvarın her katmanı kesilir", [wall(0, 4, 0, F("0.24"), plastered, fill="#5A5A5A")], [col(4, 0)]),
        ("Perde ile kolon tek kesit: perdenin dolgusu, aradaki kenar çizilmez", [w25()], [col(4, 0)]),
        ("Büyük harfle yazılmış malzeme de aynıdır", [w25(shout)], [col(4, 0, material="Betonarme")]),
        ("Çelik kolon perdeyle birleşmez, duvar yine onda durur", [w25()], [col(4, 0, material="Çelik")]),
        ("Perdenin ortasındaki kolon iki yandan birleşir", [w25()], [col(2, 0)]),
        ("Perdeden derin kolonun yalnız perdedeki kenarı çizilmez", [w25()], [column(4, 0, F("0.4"), F("0.6"))]),
        ("Tuğla duvardaki betonarme kolonun çizgisi kalır", [w20()], [col(2, 0)]),
        ("İki kolon bir duvarı üçe böler", [w20()], [col(1, 0), col(3, 0)]),
    ]
    result = []
    for name, walls, cols in out:
        result.append({
            "name": name,
            "walls": strip(walls),
            "columns": strip(cols),
            "expect": {
                "walls": [expect_wall(w, cols) for w in walls],
                "columns": [expect_column(c, walls) for c in cols],
            },
        })
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    doc = {"format": "kentos.columnJoins", "version": 1, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if a.check:
        if OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} başvuruyla aynı değil", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
