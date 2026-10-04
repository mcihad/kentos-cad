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
  - filled: the triangle tip, tip + d·h + n·h/6, tip + d·h − n·h/6;
  - open: the triangle's two sides, tip + d·h + n·h/6 → tip → tip + d·h − n·h/6;
  - dot: a circle about the tip of radius h/4;
  - none: nothing;
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


def layout(pts, height, rotation, arrow, note):
    h = height
    tip = pts[0]
    r = math.radians(rotation)
    u = (math.cos(r), math.sin(r))
    d = None
    for q in pts[1:]:
        dx, dy = q[0] - tip[0], q[1] - tip[1]
        length = math.hypot(dx, dy)
        if length > 0:
            d = (dx / length, dy / length)
            break
    if d is None:
        d = u
    n = (-d[1], d[0])
    base = (tip[0] + d[0] * h, tip[1] + d[1] * h)
    left = (base[0] + n[0] * h / 6, base[1] + n[1] * h / 6)
    right = (base[0] - n[0] * h / 6, base[1] - n[1] * h / 6)
    head = {"kind": arrow or "filled"}
    if head["kind"] == "filled":
        head["triangle"] = [tip, left, right]
    elif head["kind"] == "open":
        head["lines"] = [left, tip, right]
    elif head["kind"] == "dot":
        head["center"], head["radius"] = tip, h / 4
    side = 1
    last = pts[-1]
    for q in reversed(pts[:-1]):
        dx, dy = last[0] - q[0], last[1] - q[1]
        if math.hypot(dx, dy) > 0:
            side = 1 if dx * u[0] + dy * u[1] >= 0 else -1
            break
    out = {"head": head, "side": side}
    if note:
        end = (last[0] + side * u[0] * 2 * h, last[1] + side * u[1] * 2 * h)
        out["landing"] = [last, end]
        out["notePoint"] = (end[0] + side * u[0] * h / 2, end[1] + side * u[1] * h / 2)
        out["noteAlign"] = "middleLeft" if side > 0 else "middleRight"
    return out


# Each case: a name, the leader's vertices, height, rotation, arrowhead (None: filled) and whether it has a note.
CASES = [
    ("iki köşe, sağa giden", [(0.0, 0.0), (8.0, 6.0)], 2.5, 0.0, None, True),
    ("iki köşe, sola giden", [(30.0, 0.0), (22.0, 6.0)], 2.0, 0.0, "open", True),
    ("kırık, son parça sola", [(30.0, 0.0), (26.0, 4.0), (20.0, 6.0)], 2.0, 0.0, "open", True),
    ("30° dönük not, nokta", [(40.0, 0.0), (46.0, 8.0)], 1.5, 30.0, "dot", True),
    ("notsuz, oksuz", [(0.0, -10.0), (5.0, -6.0), (9.0, -6.0)], 2.5, 0.0, "none", False),
    ("son parça dik: izdüşüm sıfır, sağa", [(0.0, 0.0), (0.0, 5.0)], 1.0, 0.0, None, True),
    ("ilk köşe ucun üstünde: ilk uzunluklu parça", [(1.0, 1.0), (1.0, 1.0), (4.0, 5.0)], 1.0, 0.0, None, True),
    ("son köşe tekrar: önceki uzunluklu parça", [(0.0, 0.0), (-6.0, 2.0), (-6.0, 2.0)], 1.0, 0.0, None, True),
    ("bütün köşeler aynı: notun doğrultusu", [(3.0, 3.0), (3.0, 3.0)], 1.0, 90.0, None, True),
    ("büyük koordinat, 200° dönük", [(487000.125, 4420000.5), (487012.375, 4419994.25)], 3.0, 200.0, None, True),
]


def build():
    cases = []
    for name, pts, h, rot, arrow, note in CASES:
        leader = {"pts": [{"x": x, "y": y} for x, y in pts], "height": h, "rotation": rot}
        if arrow:
            leader["arrow"] = arrow
        if note:
            leader["text"] = "Not"
        got = layout(pts, h, rot, arrow, note)

        def pt(p):
            return {"x": p[0], "y": p[1]}

        head = dict(got["head"])
        for k in ("triangle", "lines"):
            if k in head:
                head[k] = [pt(p) for p in head[k]]
        if "center" in head:
            head["center"] = pt(head["center"])
        want = {"head": head, "side": got["side"]}
        if note:
            want["landing"] = [pt(p) for p in got["landing"]]
            want["notePoint"] = pt(got["notePoint"])
            want["noteAlign"] = got["noteAlign"]
        cases.append({"name": name, "leader": leader, "want": want})
    return {
        "format": "kentos.leader-cases",
        "version": 1,
        "note": "Written by scripts/fixtures/leader_cases.py from docs/adr/0146 §2 alone; the geometry core gives the same within 1e-9 m.",
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
