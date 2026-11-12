"""The outside pieces' numbers kept clear of the sheet (`cad.readjustment.sheets`' `numberLeader`, docs/adr/5080;
plan/map-cad.md MAP-11), held by the test's own reading: the font's advances by fontTools, the note's box by ADR 0145's
rule and ADR 0146's layout, the crossings by the test's own segment test (not the reference's, not KentOS).

At 6 mm on paper (6 m at 1:1000) on 101/10's two 4 m pieces:

- the leaders' lines past their pieces, their landings and notes cross no edge of the drawing's areas, and the notes do
  not overlap each other; the north number is not east (there its line would cross 101/8's west side);
- with a road's edge drawn all round 101/10, 1 m off, no place is clear: both numbers are still written, each said in a
  `number_crowded` warning, and each crosses only that road.
"""

from __future__ import annotations

import sys
import unittest
import warnings
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from fontTools.ttLib import TTFont  # noqa: E402

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import readjustment_scope_cases as rs  # noqa: E402
import readjustment_sheets_cases as sc  # noqa: E402

P = rs.P_
TEN = [P(10)]
NUMBERS = {rs.uid(P(10) + 1): ["101/24", "101/25"]}
FONT = TTFont(ROOT / "apps/desktop/assets/fonts/drawing/Barlow-500.ttf")
UPEM, CMAP, HMTX = FONT["head"].unitsPerEm, FONT.getBestCmap(), FONT["hmtx"]
ROAD = rs.area_entity(rs.rect(-9, -21, -3, 61), "yol", {"Ad": "Yol"})


def width(text: str, height: float) -> float:
    return sum(round(HMTX[CMAP[ord(c)]][0] * 1000 / UPEM) for c in text) / 1000 * height


def crosses(a, b, c, d) -> bool:
    """Whether segments ab and cd meet (touching counts)."""
    def side(p, q, r):
        v = (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        return 0 if abs(v) < 1e-12 else (1 if v > 0 else -1)
    s1, s2, s3, s4 = side(a, b, c), side(a, b, d), side(c, d, a), side(c, d, b)
    if s1 * s2 < 0 and s3 * s4 < 0:
        return True
    near = lambda p, q, r: min(p[0], q[0]) - 1e-12 <= r[0] <= max(p[0], q[0]) + 1e-12 and \
        min(p[1], q[1]) - 1e-12 <= r[1] <= max(p[1], q[1]) + 1e-12  # noqa: E731
    return (s1 == 0 and near(a, b, c)) or (s2 == 0 and near(a, b, d)) or (s3 == 0 and near(c, d, a)) or \
        (s4 == 0 and near(c, d, b))


def edges(entities) -> list[tuple[tuple, tuple, int]]:
    out = []
    for i, e in enumerate(entities):
        pts = [(q["x"], q["y"]) for q in e["pts"]]
        out += [(pts[k], pts[(k + 1) % len(pts)], i) for k in range(len(pts))]
    return out


def note(lead: dict) -> list[tuple[float, float]]:
    """ADR 0146: the landing 2 h right (the line going right or straight) or left, the note 0.5 h beyond, its box ADR
    0145's: 0.73 h under its middle, 0.65 h over, the letters' advances long."""
    tip, end = (lead["pts"][0]["x"], lead["pts"][0]["y"]), (lead["pts"][-1]["x"], lead["pts"][-1]["y"])
    h, w = lead["height"], width(lead["text"], lead["height"])
    side = 1 if end[0] - tip[0] >= 0 else -1
    x0 = end[0] + side * 2.5 * h
    x0, x1 = (x0, x0 + w) if side > 0 else (x0 - w, x0)
    return [(x0, end[1] - 0.73 * h), (x1, end[1] - 0.73 * h), (x1, end[1] + 0.65 * h), (x0, end[1] + 0.65 * h)]


def segments(lead: dict, piece: list[tuple[float, float]]) -> list[tuple[tuple, tuple]]:
    """The leader's line from where it leaves its piece (the last crossing of the piece's sides on it) through its landing."""
    tip, end = (lead["pts"][0]["x"], lead["pts"][0]["y"]), (lead["pts"][-1]["x"], lead["pts"][-1]["y"])
    h = lead["height"]
    side = 1 if end[0] - tip[0] >= 0 else -1
    dx, dy = end[0] - tip[0], end[1] - tip[1]
    best = 0.0
    for a, b in zip(piece, piece[1:] + piece[:1]):
        den = dx * (b[1] - a[1]) - dy * (b[0] - a[0])
        if den == 0:
            continue
        t = ((a[0] - tip[0]) * (b[1] - a[1]) - (a[1] - tip[1]) * (b[0] - a[0])) / den
        u = ((a[0] - tip[0]) * dy - (a[1] - tip[1]) * dx) / den
        if 0 <= u <= 1 and 0 <= t <= 1:
            best = max(best, t)
    out_at = (tip[0] + dx * (best + 1e-6), tip[1] + dy * (best + 1e-6))
    return [(out_at, end), (end, (end[0] + side * 2 * h, end[1]))]


def made(entities):
    doc = cad.Document.from_bytes(kr.container(kr.document(rs.setup(entities))))
    scope = cad.ReadjustmentScope.from_json(sc.scope_input(TEN))
    with warnings.catch_warnings(record=True) as said:
        warnings.simplefilter("always")
        out = cad.readjustment.sheets(doc, scope=scope, numbers=NUMBERS, number_height=6.0, number_leader=True)
    records = [doc.entity(u).entity.to_json() for u in out.uids]
    pieces = [[(p["x"], p["y"]) for p in r["pts"]] for r in records if r["kind"] == "polygon" and "ParselNo" in r["attrs"]]
    leaders = [r for r in records if r["kind"] == "leader"]
    return out, pieces, leaders, [str(w.message) for w in said]


def touched(lead, piece, drawing_edges) -> set[int]:
    box = note(lead)
    sides = list(zip(box, box[1:] + box[:1]))
    hit = set()
    for a, b, owner in drawing_edges:
        inside = all((q[0] - p[0]) * (a[1] - p[1]) - (q[1] - p[1]) * (a[0] - p[0]) >= 0 for p, q in sides)
        if inside or any(crosses(a, b, c, d) for c, d in sides + segments(lead, piece)):
            hit.add(owner)
    return hit


class SeparationNumberClear(unittest.TestCase):
    def test_the_numbers_outside_touch_nothing(self) -> None:
        out, pieces, leaders, said = made(rs.MAIN)
        self.assertEqual(len(leaders), 2)
        self.assertFalse([m for m in said if "konamadı" in m], said)
        drawing_edges = edges(rs.MAIN)
        for piece, lead in zip(pieces, leaders):
            self.assertEqual(touched(lead, piece, drawing_edges), set(), lead["text"])
        # The notes apart.
        a, b = note(leaders[0]), note(leaders[1])
        self.assertTrue(max(q[1] for q in a) < min(q[1] for q in b) or max(q[1] for q in b) < min(q[1] for q in a))
        # East, 101/25's line would cross 101/8's west side (x = 0 from y 49.995 to 70): the test's own check.
        north = leaders[1]
        y = north["pts"][0]["y"]
        self.assertTrue(rs.Y0 + 49.995 < y < rs.Y0 + 70)
        self.assertLess(north["pts"][-1]["x"], north["pts"][0]["x"], "not east")
        labels = out.sheets[0].labels
        self.assertEqual([lab.clear for lab in labels], [True, True])

    def test_when_every_place_touches_the_road_it_is_said(self) -> None:
        drawing = rs.MAIN + [ROAD]
        out, pieces, leaders, said = made(drawing)
        self.assertEqual(len(leaders), 2, "both still written")
        self.assertEqual(len([m for m in said if "konamadı" in m]), 2, said)
        road = len(drawing) - 1
        for piece, lead in zip(pieces, leaders):
            self.assertEqual(touched(lead, piece, edges(drawing)), {road}, lead["text"])
        labels = out.sheets[0].labels
        self.assertEqual([lab.clear for lab in labels], [False, False])
        self.assertEqual([list(lab.clashes) for lab in labels], [[rs.uid(road + 1)]] * 2)


if __name__ == "__main__":
    unittest.main()
