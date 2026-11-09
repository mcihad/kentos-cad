"""Et kalınlığı (`cad.pipes.check` `diameter.wall`; docs/adr/3960; plan/map-cad.md MAP-16) through the SDK, held by
this test's own arithmetic (no KentOS code in the expectations).

On the shared cases' walls drawing (scripts/fixtures/pipes_check_cases.py: the sewer's pipes carry `Et`, the wall in
millimetres, P7 none; the water network's too):

- the cover is measured to the pipe's top outside (Civil 3D's): each walled pipe's least and greatest cover are less
  than over its crown by its wall, exactly where they were; P7 is said (``noWall``) and keeps its crown's;
- the vertical clearance at each crossing is less than the walls-unknown one by the two pipes' walls, the horizontal one
  by the two walls too.
"""

from __future__ import annotations

import json
import sys
import unittest
import warnings
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.pipes.check.json").read_text("utf-8"))
SETUP = next(c for c in CASES["cases"] if c["name"].startswith("et kalınlığı: örtü"))["setup"]
SEWER_IN = cad.PipeInverts.from_json({"from": "fields", "start": "Akar baş", "end": "Akar son"})
ZEMIN = cad.PipeGround.from_json({"from": "surface", "surface": "Zemin"})


def diameter(wall: bool) -> cad.PipeDiameter:
    return cad.PipeDiameter(field="Çap", unit=cad.PipeUnit.MM, wall="Et" if wall else cad.UNSET)


def walls() -> dict[str, float]:
    """Each pipe's wall in metres, read from the drawing by this test."""
    return {u: float(e["attrs"]["Et"]) / 1000.0 for u, e in zip(SETUP["uids"], SETUP["entities"]) if "Et" in e.get("attrs", {})}


class PipeWalls(unittest.TestCase):
    def setUp(self) -> None:
        warnings.simplefilter("ignore", cad.CommandWarning)
        self.doc = cad.Document.from_bytes(kr.container(kr.document(SETUP)))

    def check(self, wall: bool, clearance=None) -> cad.PipesChecked:
        return cad.pipes.check(self.doc, network="atiksu", inverts=SEWER_IN, diameter=diameter(wall), ground=ZEMIN,
                               clearance=clearance if clearance is not None else cad.UNSET)

    def test_the_cover_is_to_the_top_outside(self) -> None:
        inner, outer, w = self.check(False), self.check(True), walls()
        self.assertEqual((inner.cover_to, outer.cover_to), ("crown", "outside"))
        for a, b in zip(inner.pipes, outer.pipes):
            if a.min_cover is cad.UNSET:
                continue
            if b.uid in w:
                self.assertAlmostEqual(a.min_cover - b.min_cover, w[b.uid], places=9)
                self.assertAlmostEqual(a.max_cover - b.max_cover, w[b.uid], places=9)
                self.assertEqual(a.min_cover_at, b.min_cover_at)
                self.assertAlmostEqual(b.wall, w[b.uid], places=12)
            else:
                self.assertIn("noWall", b.findings)
                self.assertEqual(a.min_cover, b.min_cover)

    def test_the_clearances_are_outside_to_outside(self) -> None:
        ask = lambda wall: cad.PipeClearanceAsk(network="su", inverts=cad.PipeInverts.from_json({"from": "heights"}),
                                               diameter=diameter(wall), horizontal=3.0)
        inner, outer, w = self.check(False, ask(False)), self.check(True, ask(True)), walls()
        pairs = 0
        for a, b in zip(inner.crossings, outer.crossings):
            self.assertEqual((a.pipe, a.other), (b.pipe, b.other))
            if a.clearance is cad.UNSET:
                continue
            self.assertAlmostEqual(a.clearance - b.clearance, w.get(a.pipe, 0.0) + w.get(a.other, 0.0), places=9)
            pairs += 1
        self.assertGreater(pairs, 2)
        for a, b in zip(inner.near, outer.near):
            self.assertAlmostEqual(a.clearance - b.clearance, w.get(a.pipe, 0.0) + w.get(a.other, 0.0), places=9)


if __name__ == "__main__":
    unittest.main()
