"""The outside pieces' numbers fitted inside them (`cad.readjustment.sheets`' `numberHeight`, docs/adr/4280; plan/map-cad.md
MAP-11), held by the test's own reading: the font's advances by fontTools (not the reference's hand reader, not KentOS),
the text's box by ADR 0145's rule, inside by the box's corners in the piece (the pieces here are convex).

- At 2 mm on paper the numbers are texts, not labelled points; 101/10's two pieces, 4 m wide and 10 m tall, take their
  numbers turned 90° along their long sides at the full height; every text's box lies inside its piece.
- At 6 mm the 4 m pieces' numbers step down to the largest tenth whose box fits, which the test finds on its own.
- Without `numberHeight` the numbers stay labelled points (docs/adr/3480).
"""

from __future__ import annotations

import math
import sys
import unittest
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


def width(text: str, height: float) -> float:
    return sum(round(HMTX[CMAP[ord(c)]][0] * 1000 / UPEM) for c in text) / 1000 * height


def corners(t: dict) -> list[tuple[float, float]]:
    """ADR 0145's box of a middle-centred one-line text: half its width each way, 0.73 of its height under its middle
    and 0.65 over it, turned with it."""
    h, w = t["height"], width(t["text"], t["height"])
    r = math.radians(t["rotation"])
    c, s = math.cos(r), math.sin(r)
    x0, y0 = t["p"]["x"], t["p"]["y"]
    return [(x0 + c * x - s * y, y0 + s * x + c * y) for x, y in ((-w / 2, -0.73 * h), (w / 2, -0.73 * h),
                                                                 (w / 2, 0.65 * h), (-w / 2, 0.65 * h))]


def inside(q: tuple[float, float], ring: list[tuple[float, float]]) -> bool:
    """A point inside or on a convex ring, whichever way it runs."""
    sides = [(b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
             for a, b in zip(ring, ring[1:] + ring[:1])]
    return all(v >= -1e-9 for v in sides) or all(v <= 1e-9 for v in sides)


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(rs.setup(rs.MAIN))))


class SeparationNumberFit(unittest.TestCase):
    def written(self, height: float | None) -> tuple[list[dict], list[dict], object]:
        doc = drawing()
        scope = cad.ReadjustmentScope.from_json(sc.scope_input(TEN))
        out = cad.readjustment.sheets(doc, scope=scope, numbers=NUMBERS,
                                      **({"number_height": height} if height is not None else {}))
        records = [doc.entity(u).entity.to_json() for u in out.uids]
        return [r for r in records if r["kind"] == "polygon"], [r for r in records if r["kind"] == "text"], out

    def test_at_2_mm_the_numbers_are_texts_along_the_narrow_pieces(self) -> None:
        areas, texts, out = self.written(2.0)
        scale = out.sheets[0].scale
        self.assertEqual(sorted(t["text"] for t in texts), ["101/24", "101/25"])
        for t in texts:
            self.assertAlmostEqual(t["height"], 2.0 * scale / 1000, delta=1e-9)
            self.assertAlmostEqual(abs(t["rotation"]), 90.0, delta=1e-9)
            self.assertEqual(t["attrs"]["ParselNo"], t["text"])
            piece = next(a for a in areas if a["attrs"]["ParselNo"] == t["text"])
            ring = [(p["x"], p["y"]) for p in piece["pts"]]
            self.assertTrue(all(inside(q, ring) for q in corners(t)), t)

    def test_at_6_mm_the_numbers_step_down_to_the_largest_tenth_that_fits(self) -> None:
        areas, texts, out = self.written(6.0)
        asked = 6.0 * out.sheets[0].scale / 1000
        for t in texts:
            piece = next(a for a in areas if a["attrs"]["ParselNo"] == t["text"])
            ring = [(p["x"], p["y"]) for p in piece["pts"]]
            share = round(t["height"] / asked, 9)
            self.assertTrue(all(inside(q, ring) for q in corners(t)), t)
            bigger = dict(t, height=asked * (share + 0.1))
            self.assertFalse(all(inside(q, ring) for q in corners(bigger)), f"{t['text']}: {share + 0.1} fits too")
            label = next(l for l in out.sheets[0].labels if l.number == t["text"])
            self.assertAlmostEqual(label.share, share, delta=1e-9)
            self.assertTrue(label.fits)

    def test_without_a_height_the_numbers_stay_labelled_points(self) -> None:
        doc = drawing()
        out = cad.readjustment.sheets(doc, scope=cad.ReadjustmentScope.from_json(sc.scope_input(TEN)), numbers=NUMBERS)
        records = [doc.entity(u).entity.to_json() for u in out.uids]
        self.assertFalse(any(r["kind"] == "text" for r in records))
        self.assertEqual(sorted(r["label"] for r in records if r["kind"] == "point" and r.get("label", "").startswith("101/")),
                         ["101/24", "101/25"])


if __name__ == "__main__":
    unittest.main()
