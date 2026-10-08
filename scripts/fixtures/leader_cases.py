#!/usr/bin/env python3
"""Independent reference of a leader's layout (docs/adr/0146 §2).

Writes fixtures/leader/v1/layout.json from the rule alone, with Python's
standard library and no KentOS code: for each leader, where its arrowhead,
its landing and its note go. The geometry core (`geom::leader::layout`)
must give the same within 1e-9 m on both platforms
(crates/shared/geometry-core/tests/all/leader.rs; the web through its WASM).

The rule, every length in the note's height h:

- the arrowhead points from the tip (the first vertex) along the first
  segment that has a length: d is its direction, n = d turned a quarter
  counter-clockwise; when every vertex is the tip, d is the note's direction;
- its length L is the leader's arrowSize (none: 1) times h; in its own frame
  x runs from the tip along d and y along n, a point (x, y) being
  tip + d·x·L + n·y·L (docs/adr/0205 §7):
  - closed filled (no arrow): the area (0, 0), (1, 1/6), (1, −1/6), its back 1;
  - closed (blank): those three as a closed line, its back 1;
  - open: the line (1, 1/6), (0, 0), (1, −1/6); open30: (1, t), (0, 0),
    (1, −t) with t = tan 15°; open90: (½, ½), (0, 0), (½, −½); their back 0;
  - dot: the area of the circle of radius ¼ about the tip, its back ¼;
    dotSmall: radius ⅛, its back ⅛; dotBlank: the radius ¼ circle as a closed
    line, its back ¼; a circle is 72 points, the i-th at angle 2πi/72 from x;
  - oblique: the line (−½, −½), (½, ½), its back 0; archTick: that line as a
    band an eighth wide, the area (−½ + w, −½ − w), (½ + w, ½ − w),
    (½ − w, ½ + w), (−½ − w, −½ + w) with w = 1/(16·√2), its back 0;
  - boxFilled: the area (−¼, −¼), (¼, −¼), (¼, ¼), (−¼, ¼), its back ¼;
    boxBlank: that square as a closed line;
  - datumFilled: the area (0, −½), (1, 0), (0, ½), its back 1;
  - none: nothing, its back 0;
- the line starts at the tip (its first vertex the second) when the back is
  0; else on the first segment that has a length, the back from the tip
  (that segment's end its first vertex) when the segment is longer than the
  back, at that segment's end otherwise (the vertex after it its first);
- u is the note's direction (its rotation, degrees counter-clockwise from
  east); the last segment that has a length goes to the right (its
  projection on u is 0 or more: side +1) or to the left (side −1); with no
  such segment, the right;
- with a note, the landing runs from the last vertex 2h along side·u, and
  the note's point is h/2 past its end, aligned on the middle of its left
  (side +1) or of its right (side −1); without a note, neither.

    python3 scripts/fixtures/leader_cases.py           # writes the cases
    python3 scripts/fixtures/leader_cases.py --check   # compares with the file
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/leader/v1/layout.json"


def circle(r):
    return [(r * math.cos(2 * math.pi * i / 72), r * math.sin(2 * math.pi * i / 72)) for i in range(72)]


def head_of(arrow):
    """The arrowhead's areas, lines (points, closed) and back, in its frame (x along the line, y across), in L."""
    t = math.tan(math.pi / 12)
    tri = [(0, 0), (1, 1 / 6), (1, -1 / 6)]
    sq = [(-0.25, -0.25), (0.25, -0.25), (0.25, 0.25), (-0.25, 0.25)]
    w = 1 / 16 / math.sqrt(2)
    return {
        None: ([tri], [], 1),
        "closed": ([], [(tri, True)], 1),
        "open": ([], [([(1, 1 / 6), (0, 0), (1, -1 / 6)], False)], 0),
        "open30": ([], [([(1, t), (0, 0), (1, -t)], False)], 0),
        "open90": ([], [([(0.5, 0.5), (0, 0), (0.5, -0.5)], False)], 0),
        "dot": ([circle(0.25)], [], 0.25),
        "dotSmall": ([circle(0.125)], [], 0.125),
        "dotBlank": ([], [(circle(0.25), True)], 0.25),
        "oblique": ([], [([(-0.5, -0.5), (0.5, 0.5)], False)], 0),
        "archTick": ([[(-0.5 + w, -0.5 - w), (0.5 + w, 0.5 - w), (0.5 - w, 0.5 + w), (-0.5 - w, -0.5 + w)]], [], 0),
        "boxFilled": ([sq], [], 0.25),
        "boxBlank": ([], [(sq, True)], 0.25),
        "datumFilled": ([[(0, -0.5), (1, 0), (0, 0.5)]], [], 1),
        "none": ([], [], 0),
    }[arrow]


def layout(pts, height, rotation, arrow, size, note):
    h = height
    tip = pts[0]
    r = math.radians(rotation)
    u = (math.cos(r), math.sin(r))
    lead = None
    for i, q in enumerate(pts[1:], start=1):
        dx, dy = q[0] - tip[0], q[1] - tip[1]
        length = math.hypot(dx, dy)
        if length > 0:
            lead = (i, (dx / length, dy / length), length)
            break
    d = lead[1] if lead else u
    n = (-d[1], d[0])
    L = (size or 1) * h

    def at(p):
        return (tip[0] + d[0] * p[0] * L + n[0] * p[1] * L, tip[1] + d[1] * p[0] * L + n[1] * p[1] * L)

    fills, lines, back = head_of(arrow)
    head = {"fills": [[at(p) for p in ring] for ring in fills], "lines": [{"pts": [at(p) for p in ps], "closed": c} for ps, c in lines], "back": back * L}
    if lead and back > 0 and back * L < lead[2]:
        start, first = (tip[0] + lead[1][0] * back * L, tip[1] + lead[1][1] * back * L), lead[0]
    elif lead and back > 0:
        start, first = pts[lead[0]], lead[0] + 1
    else:
        start, first = tip, 1
    side = 1
    last = pts[-1]
    for q in reversed(pts[:-1]):
        dx, dy = last[0] - q[0], last[1] - q[1]
        if math.hypot(dx, dy) > 0:
            side = 1 if dx * u[0] + dy * u[1] >= 0 else -1
            break
    out = {"head": head, "start": start, "first": first, "side": side}
    if note:
        end = (last[0] + side * u[0] * 2 * h, last[1] + side * u[1] * 2 * h)
        out["landing"] = [last, end]
        out["notePoint"] = (end[0] + side * u[0] * h / 2, end[1] + side * u[1] * h / 2)
        out["noteAlign"] = "middleLeft" if side > 0 else "middleRight"
    return out


# Each case: a name, the leader's vertices, height, rotation, arrowhead (None: filled), its size (None: 1) and whether it has a note.
CASES = [
    ("iki köşe, sağa giden", [(0.0, 0.0), (8.0, 6.0)], 2.5, 0.0, None, None, True),
    ("iki köşe, sola giden", [(30.0, 0.0), (22.0, 6.0)], 2.0, 0.0, "open", None, True),
    ("kırık, son parça sola", [(30.0, 0.0), (26.0, 4.0), (20.0, 6.0)], 2.0, 0.0, "open", None, True),
    ("30° dönük not, nokta", [(40.0, 0.0), (46.0, 8.0)], 1.5, 30.0, "dot", None, True),
    ("notsuz, oksuz", [(0.0, -10.0), (5.0, -6.0), (9.0, -6.0)], 2.5, 0.0, "none", None, False),
    ("son parça dik: izdüşüm sıfır, sağa", [(0.0, 0.0), (0.0, 5.0)], 1.0, 0.0, None, None, True),
    ("ilk köşe ucun üstünde: ilk uzunluklu parça", [(1.0, 1.0), (1.0, 1.0), (4.0, 5.0)], 1.0, 0.0, None, None, True),
    ("son köşe tekrar: önceki uzunluklu parça", [(0.0, 0.0), (-6.0, 2.0), (-6.0, 2.0)], 1.0, 0.0, None, None, True),
    ("bütün köşeler aynı: notun doğrultusu", [(3.0, 3.0), (3.0, 3.0)], 1.0, 90.0, None, None, True),
    ("büyük koordinat, 200° dönük", [(487000.125, 4420000.5), (487012.375, 4419994.25)], 3.0, 200.0, None, None, True),
    ("boş üçgen, ok boyu 1,5", [(0.0, 0.0), (6.0, 8.0), (12.0, 8.0)], 2.0, 0.0, "closed", 1.5, True),
    ("ince açık ok", [(0.0, 0.0), (-3.0, 4.0)], 2.0, 0.0, "open30", None, True),
    ("dik açık ok, ok boyu 0,5", [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)], 2.0, 0.0, "open90", 0.5, False),
    ("küçük nokta", [(10.0, 10.0), (14.0, 13.0)], 1.0, 0.0, "dotSmall", None, True),
    ("boş nokta, ok boyu 2", [(10.0, 10.0), (10.0, 20.0)], 1.0, 0.0, "dotBlank", 2.0, True),
    ("eğik çizgi", [(0.0, 0.0), (8.0, 0.0)], 2.5, 0.0, "oblique", None, True),
    ("mimari çentik", [(0.0, 0.0), (0.0, -8.0)], 2.5, 0.0, "archTick", None, True),
    ("dolu kare", [(5.0, 5.0), (9.0, 8.0)], 2.0, 0.0, "boxFilled", None, True),
    ("boş kare, ok boyu 0,1", [(5.0, 5.0), (9.0, 8.0)], 2.0, 0.0, "boxBlank", 0.1, True),
    ("dayanak üçgeni", [(0.0, 0.0), (6.0, -8.0)], 2.0, 15.0, "datumFilled", None, True),
    ("ilk parça okun boyundan kısa: çizgi köşesinden başlar", [(0.0, 0.0), (1.0, 0.0), (1.0, 6.0)], 2.0, 0.0, None, None, True),
    ("ok boyu 10, büyük koordinat", [(487000.125, 4420000.5), (487040.375, 4420030.25)], 1.5, 0.0, None, 10.0, True),
]


def build():
    cases = []
    for name, pts, h, rot, arrow, size, note in CASES:
        leader = {"pts": [{"x": x, "y": y} for x, y in pts], "height": h, "rotation": rot}
        if arrow:
            leader["arrow"] = arrow
        if size is not None:
            leader["arrowSize"] = size
        if note:
            leader["text"] = "Not"
        got = layout(pts, h, rot, arrow, size, note)

        def pt(p):
            return {"x": p[0], "y": p[1]}

        head = got["head"]
        want = {
            "head": {
                "fills": [[pt(p) for p in ring] for ring in head["fills"]],
                "lines": [{"pts": [pt(p) for p in line["pts"]], "closed": line["closed"]} for line in head["lines"]],
                "back": head["back"],
            },
            "start": pt(got["start"]),
            "first": got["first"],
            "side": got["side"],
        }
        if note:
            want["landing"] = [pt(p) for p in got["landing"]]
            want["notePoint"] = pt(got["notePoint"])
            want["noteAlign"] = got["noteAlign"]
        cases.append({"name": name, "leader": leader, "want": want})
    return {
        "format": "kentos.leader-cases",
        "version": 2,
        "note": "Written by scripts/fixtures/leader_cases.py from docs/adr/0146 §2 and 0205 §7 alone; the geometry core gives the same within 1e-9 m.",
        "cases": cases,
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)}: kurallardan yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(CASES)} durum kurallarla aynı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(CASES)} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
