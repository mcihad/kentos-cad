"""Pafta eğrileri kept as a drawing and drawn again writing only what changed; BÖHHBÜY m. 63(2)'s heights in built-up
areas and along roads; the chosen points' heights against m. 93(1)(c)'s limit (`road.terrains.sheetcontours`,
docs/adr/4740; plan/map-cad.md MAP-14), held by this test's own readings of the shared cases' drawing
(scripts/fixtures/road_sheet_contour_cases.py writes it).

- m. 63(2) at 1/1000: every road height lies on the road's centre line, the first 10 m from its start and each next one
  20 m (2 cm on the sheet) further; every built-up height is on the 20 m grid, inside the built-up area, outside the
  building and at least 10 m from every other height.
- m. 93(1)(c): the check's RMS is the test's own over the chosen points' own heights against the surface's (the one on a
  surface corner left out), its limit 1.5 × 1.33 × KOH_X,Y of m. 91(2) for S = 1000.
- Drawn, the drawing is kept with the surface and the layers it read; a ground height raised makes it stale; drawn
  again with `regional`, the lines alike keep their ids, the heights after a written line are written again, and the
  drawing is current; a project told to draw whole says so in its tree.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/road.terrains.sheetcontours.json").read_text("utf-8"))
CMD = cad.road.terrains.sheetcontours
UIDS = CASES["setup"]["uids"]


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))


def entity(kind: str, layer: str) -> dict:
    return next(e for e in CASES["setup"]["entities"] if e["kind"] == kind and e["layerId"] == layer)


def inside(ring: list[tuple[float, float]], p: tuple[float, float]) -> bool:
    odd = False
    for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
        if (ay > p[1]) != (by > p[1]) and p[0] < ax + (p[1] - ay) * (bx - ax) / (by - ay):
            odd = not odd
    return odd


class SheetBuiltUp(unittest.TestCase):
    def test_the_heights_are_two_centimetres_apart_on_the_sheet(self) -> None:
        doc = drawing()
        built = {"areas": ["yerlesik"], "roads": ["yol-ekseni"]}
        out = CMD.plan_input(doc, cad.TerrainsSheetContours.from_json(
            {"surface": "t-1", "layerId": "esyukselti", "builtUp": built, "details": ["bina"]}))
        axis = [(p["x"], p["y"]) for p in entity("polyline", "yol-ekseni")["pts"]]
        (ax, ay), (bx, by) = axis
        length = math.hypot(bx - ax, by - ay)
        roads = [s for s in out.spots if s.kind == "road"]
        self.assertGreater(len(roads), 10)
        for s in roads:
            # On the line: no distance across it.
            across = ((bx - ax) * (s.y - ay) - (by - ay) * (s.x - ax)) / length
            self.assertLess(abs(across), 1e-9)
        along = sorted(math.hypot(s.x - ax, s.y - ay) for s in roads)
        # The first places (and maybe the last) fall off the surface: those left are 10 m + 20 m·k from the start.
        for d in along:
            k = (d - 10.0) / 20.0
            self.assertAlmostEqual(k, round(k), delta=1e-9)
        area = [(p["x"], p["y"]) for p in entity("polygon", "yerlesik")["pts"]]
        building = [(p["x"], p["y"]) for p in entity("polygon", "bina")["pts"]]
        grid = [s for s in out.spots if s.kind == "builtUp"]
        self.assertGreater(len(grid), 10)
        for s in grid:
            self.assertEqual((s.x % 20.0, s.y % 20.0), (0.0, 0.0))
            self.assertTrue(inside(area, (s.x, s.y)))
            self.assertFalse(inside(building, (s.x, s.y)))
            others = [o for o in out.spots if o is not s]
            self.assertTrue(all(math.hypot(o.x - s.x, o.y - s.y) >= 10.0 for o in others))
        note = next(n for n in out.notes if n.startswith("Yollarda"))
        self.assertIn(f"Yollarda {len(roads)}, yerleşik alanlarda {len(grid)} kot", note)


class SectionCheck(unittest.TestCase):
    def test_the_rms_and_the_limit_are_the_test_s_own(self) -> None:
        doc = drawing()
        out = CMD.plan_input(doc, cad.TerrainsSheetContours.from_json(
            {"surface": "t-1", "layerId": "esyukselti", "points": ["kontrol-asan"]}))
        chosen = [s for s in out.spots if s.kind == "chosen" and s.surface not in (None, cad.UNSET)]
        self.assertEqual(len(chosen), 5)
        # The ground point's own place is a corner of the surface: its difference is the 0.02 m put on it, left out.
        ground = {(e["p"]["x"], e["p"]["y"]) for e in CASES["setup"]["entities"] if e["layerId"] == "zemin"}
        compared = [s for s in chosen if (s.x, s.y) not in ground]
        self.assertEqual(len(compared), 4)
        rms = math.sqrt(sum((s.z - s.surface) ** 2 for s in compared) / len(compared))
        s_ = 1000.0
        koh_xy = -1.665e-6 * s_ * s_ + 0.01745 * s_ - 1.166
        limit = 1.5 * 1.33 * koh_xy / 100.0
        c = out.check
        self.assertEqual((c.compared, c.corners), (4, 1))
        self.assertAlmostEqual(c.rms, rms, delta=1e-12)
        self.assertAlmostEqual(c.limit, limit, delta=1e-12)
        self.assertFalse(c.within)
        self.assertGreater(rms, limit)


class KeptDrawing(unittest.TestCase):
    def test_a_raised_height_makes_it_stale_and_only_what_changed_is_written(self) -> None:
        doc = drawing()
        CMD.execute_input(doc, cad.TerrainsSheetContours.from_json(
            {"surface": "t-1", "layerId": "esyukselti", "details": ["bina", "yol", "sev"]}))
        tree = cad.road.project.query.execute_input(doc, cad.RoadProjectQuery.from_json({}))
        node = next(n for n in tree.nodes if n.kind == "drawing")
        self.assertEqual(node.state, "current")
        self.assertTrue(tree.regional, "Yalnız değişeni yaz is the project's first choice")
        old = {r.uid: r.entity.to_json() for r in doc.entities(layer="esyukselti")}
        # The rebuild case's raised point.
        case = next(c for c in CASES["cases"] if c["name"].startswith("Kayıtlı çizim: zeminin"))
        text = json.dumps(case["steps"][1]["input"])
        for slot in range(len(UIDS), 0, -1):
            text = text.replace(f'"$uidOf:{slot}"', json.dumps(UIDS[slot - 1]))
        self.assertEqual(doc.run("cad.entities.edit", json.loads(text))["status"], "completed")
        tree = cad.road.project.query.execute_input(doc, cad.RoadProjectQuery.from_json({}))
        node = next(n for n in tree.nodes if n.kind == "drawing")
        self.assertEqual(node.state, "stale")
        self.assertTrue(any("yüzeyi" in w for w in node.why), node.why)
        made = cad.road.project.rebuild.execute_input(doc, cad.RoadProjectRebuild.from_json({"regional": True}))
        (r,) = made.rebuilt
        kept = [u for u in r.uids if u in old]
        self.assertEqual(r.kept, len(kept))
        self.assertGreater(len(kept), 0)
        now = {u: doc.entity(u).entity.to_json() for u in r.uids}
        # What stayed is a line alike its old self; every height text after the first written line is new.
        self.assertTrue(all(now[u]["kind"] == "polyline" for u in kept))
        self.assertTrue(all(now[u] == old[u] for u in kept))
        first_written = next(i for i, u in enumerate(r.uids) if u not in old)
        self.assertTrue(all(u not in old for u in r.uids[first_written:] if now[u]["kind"] == "text"))
        tree = cad.road.project.query.execute_input(doc, cad.RoadProjectQuery.from_json({}))
        self.assertEqual(next(n for n in tree.nodes if n.kind == "drawing").state, "current")
        edited = cad.road.project.edit.execute_input(doc, cad.RoadProjectEdit.from_json({"regional": False}))
        self.assertFalse(edited.regional)
        self.assertFalse(cad.road.project.query.execute_input(doc, cad.RoadProjectQuery.from_json({})).regional)


if __name__ == "__main__":
    unittest.main()
