"""The shared cases of Hizala ve dağıt (docs/adr/0194 §2).

    python3 scripts/fixtures/arrange_cases.py           # writes the file
    python3 scripts/fixtures/arrange_cases.py --check   # writes nothing; compares

Writes fixtures/arrange/v1/cases.json: objects' boxes, a mode and, for the
six alignments, the easting or northing to meet (`at`), and each box's
displacement. Written from the ADR, without KentOS code:

- a reference box's `at`: its west side (left), middle (center), east side
  (right), north side (top), middle (middle) or south side (bottom); the
  selection's box is the union of the boxes;
- an alignment moves each box along one axis only: left `at - min_x`,
  center `at - (min_x + max_x) / 2`, right `at - max_x`, top
  `at - max_y`, middle `at - (min_y + max_y) / 2`, bottom `at - min_y`;
- a spread orders the boxes by their middles along the axis (ties in the
  given order); the first and the last stay; `gap = ((last.max -
  first.min) - sizes) / (n - 1)`, `sizes` summed in that order; the k-th
  box starts at `start(k-1) + size(k-1) + gap` and moves `start(k) - min`.

The values are IEEE doubles in that order of operations (the core must
meet them bit for bit); each is also checked against the exact rational
answer of the same rule (`fractions`), within 1e-8 m (a few units in the
last place of a coordinate near 4.4 million). Fewer than three
boxes spread nothing.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/arrange/v1/cases.json"
E, N = 487000.0, 4420000.0
ALIGNS = ("left", "center", "right", "top", "middle", "bottom")
SPREADS = ("horizontal", "vertical")


def box(x0, y0, x1, y1):
    """A box east and north of (487000, 4420000)."""
    return [E + x0, N + y0, E + x1, N + y1]


def at_of(b, mode):
    """A reference box's side or middle the mode meets."""
    x0, y0, x1, y1 = b
    return {"left": x0, "center": (x0 + x1) / 2, "right": x1, "top": y1, "middle": (y0 + y1) / 2, "bottom": y0}[mode]


def union(boxes):
    return [min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes)]


def align(boxes, mode, at):
    out = []
    for b in boxes:
        d = at - at_of(b, mode)
        out.append([d, 0.0] if mode in ("left", "center", "right") else [0.0, d])
    return out


def spread(boxes, mode):
    n = len(boxes)
    if n < 3:
        return [[0.0, 0.0] for _ in boxes]
    lo, hi = (0, 2) if mode == "horizontal" else (1, 3)
    order = sorted(range(n), key=lambda i: (boxes[i][lo] + boxes[i][hi]) / 2)
    first, last = boxes[order[0]], boxes[order[-1]]
    sizes = 0.0
    for i in order:
        sizes += boxes[i][hi] - boxes[i][lo]
    gap = ((last[hi] - first[lo]) - sizes) / (n - 1)
    out = [[0.0, 0.0] for _ in boxes]
    start = first[lo]
    for k in range(1, n - 1):
        prev = boxes[order[k - 1]]
        start = (start + (prev[hi] - prev[lo])) + gap
        b = boxes[order[k]]
        d = start - b[lo]
        out[order[k]] = [d, 0.0] if mode == "horizontal" else [0.0, d]
    return out


def exact(boxes, mode, at):
    """The same rule in rationals."""
    q = [[Fraction(v) for v in b] for b in boxes]
    if mode in ALIGNS:
        a = Fraction(at)
        side = {"left": lambda b: b[0], "center": lambda b: (b[0] + b[2]) / 2, "right": lambda b: b[2],
                "top": lambda b: b[3], "middle": lambda b: (b[1] + b[3]) / 2, "bottom": lambda b: b[1]}[mode]
        return [[a - side(b), Fraction(0)] if mode in ("left", "center", "right") else [Fraction(0), a - side(b)] for b in q]
    n = len(q)
    if n < 3:
        return [[Fraction(0), Fraction(0)] for _ in q]
    lo, hi = (0, 2) if mode == "horizontal" else (1, 3)
    order = sorted(range(n), key=lambda i: (q[i][lo] + q[i][hi]) / 2)
    first, last = q[order[0]], q[order[-1]]
    gap = ((last[hi] - first[lo]) - sum(q[i][hi] - q[i][lo] for i in order)) / (n - 1)
    out = [[Fraction(0), Fraction(0)] for _ in q]
    start = first[lo]
    for k in range(1, n - 1):
        prev = q[order[k - 1]]
        start = start + (prev[hi] - prev[lo]) + gap
        d = start - q[order[k]][lo]
        out[order[k]] = [d, Fraction(0)] if mode == "horizontal" else [Fraction(0), d]
    return out


def moves(boxes, mode, at):
    got = align(boxes, mode, at) if mode in ALIGNS else spread(boxes, mode)
    want = exact(boxes, mode, at)
    for g, w in zip(got, want):
        for a, b in zip(g, w):
            assert abs(Fraction(a) - b) <= Fraction(1, 10**8), (mode, g, w)
    return got


# A parcel, a symbol's box, a long text's, a point's (no extent) and an overlapping block's.
PARCEL = box(-40, -20, -10, 10)
SYMBOL = box(4.25, 2.5, 5.75, 4)
TEXT = box(12, -6.4, 31.6, -4.1)
POINT = box(-3, 7, -3, 7)
BLOCK = box(-14, -2.5, 2.5, 8.75)
FIVE = [PARCEL, SYMBOL, TEXT, POINT, BLOCK]

CASES = []


def case(name, mode, boxes, at=None, reference=None):
    """`reference`: how `at` was given (a box's side, the selection's box, a point)."""
    if mode in ALIGNS and at is None:
        raise SystemExit(f"{name}: an alignment needs `at`")
    CASES.append({"name": name, "mode": mode, "at": at, "reference": reference, "boxes": boxes, "moves": moves(boxes, mode, at)})


for mode in ALIGNS:
    sel = union(FIVE)
    case(f"{mode}: seçimin kutusuna", mode, FIVE, at_of(sel, mode), "selection")
    case(f"{mode}: parselin kutusuna", mode, FIVE, at_of(PARCEL, mode), "box")
case("left: noktaya (487012.125)", "left", FIVE, E + 12.125, "point")
case("top: noktaya (4420003.3)", "top", FIVE, N + 3.3, "point")
case("center: tek nesne", "center", [TEXT], E + 0.1, "point")
case("middle: kesirli kutular", "middle", [box(0.1, 0.2, 0.3, 0.7), box(1.1, -0.35, 2.2, 0.05), box(5, 3.3, 9, 3.3)], N + 0.15, "point")

case("horizontal: beş kutu", "horizontal", FIVE)
case("vertical: beş kutu", "vertical", FIVE)
case("horizontal: üç kutu, sıra ortalara göre", "horizontal", [box(30, 0, 40, 5), box(0, 0, 2, 5), box(9, 0, 21, 5)])
case("horizontal: örtüşen kutular, eksi aralık", "horizontal", [box(0, 0, 10, 1), box(4, 0, 16, 1), box(8, 0, 18, 1)])
case("horizontal: eşit ortalar seçimin sırasıyla", "horizontal", [box(0, 0, 2, 1), box(10, 0, 12, 1), box(9, 0, 13, 1), box(20, 0, 21, 1)])
case("vertical: dört kutu, kesirli aralık", "vertical", [box(0, 0, 1, 0.3), box(0, 0.7, 1, 1.1), box(0, 3.05, 1, 3.4), box(0, 9.9, 1, 10)])
case("horizontal: iki kutu dağıtılmaz", "horizontal", [box(0, 0, 1, 1), box(5, 0, 7, 1)])

REFERENCES = [{"box": b, "mode": m, "at": at_of(b, m)} for b in (PARCEL, TEXT, POINT) for m in ALIGNS]


def build():
    data = {
        "format": "kentos.arrange-cases",
        "version": 1,
        "source": "scripts/fixtures/arrange_cases.py (docs/adr/0194 §2)",
        "note": "Kutular [batı, güney, doğu, kuzey]; kaymalar [doğu, kuzey], IEEE double, ADR'deki işlem sırasıyla.",
        "references": REFERENCES,
        "unions": [{"boxes": FIVE, "box": union(FIVE)}],
        "cases": CASES,
    }
    return json.dumps(data, ensure_ascii=False, indent=1) + "\n"


def main():
    text = build()
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            raise SystemExit(f"{OUT.relative_to(ROOT)}: yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
        print(f"{OUT.relative_to(ROOT)}: {len(CASES)} durum, {len(REFERENCES)} başvuru tutuyor")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(CASES)} durum")


if __name__ == "__main__":
    main()
