"""Material zones in a surface comparison (`road.terrains.compare` with `zones`, docs/adr/3540; plan/map-cad.md MAP-15)
through the SDK, held by the shared case file scripts/fixtures/road_terrain_compare_zone_cases.py writes (each zone met
in GEOS, the integral from the parts' centroids; no KentOS code in the expectations).

- Rock, earth and clay over the terrace: each zone's part, the rest and the quantity by material are the case's; the
  zones and the rest add up to the whole difference.
- Two zones sharing ground are refused at the second one's boundary; the drawing is not touched (the call reads only).
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/road.terrains.compare.zones.json").read_text())
CMD = cad.road.terrains.compare


def drawing() -> cad.Document:
    # The case's drawing is the file's v2 form: written by the independent KCAD v2 writer, then opened as a file.
    return cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))


def step(prefix: str) -> dict:
    return next(c for c in CASES["cases"] if c["name"].startswith(prefix))["steps"][0]


class CompareMaterialZones(unittest.TestCase):
    def test_three_materials_are_the_reading_s(self) -> None:
        s = step("Üç bölge")
        out = CMD.plan_input(drawing(), cad.TerrainsCompare.from_json(s["input"]))
        want = s["result"]["output"]
        self.assertEqual([z.name for z in out.zones], [z["name"] for z in want["zones"]])
        for got, w in zip(out.zones, want["zones"]):
            for key, attr in (("area", "area"), ("covered", "covered"), ("cut", "cut"), ("fill", "fill"),
                              ("cutArea", "cut_area"), ("fillArea", "fill_area")):
                self.assertAlmostEqual(getattr(got, attr), w[key], places=9, msg=f"{got.name} {key}")
            self.assertEqual((got.adjusted.cut_factor, got.adjusted.fill_factor),
                             (w["adjusted"]["cutFactor"], w["adjusted"]["fillFactor"]))
            self.assertAlmostEqual(got.adjusted.net, w["adjusted"]["net"], places=9)
        self.assertAlmostEqual(out.rest.cut, want["rest"]["cut"], places=9)
        self.assertAlmostEqual(out.rest.fill, want["rest"]["fill"], places=9)
        self.assertAlmostEqual(out.zoned.net, want["zoned"]["net"], places=9)
        # The zones and the rest are the whole difference.
        self.assertAlmostEqual(sum(z.cut for z in out.zones) + out.rest.cut, out.cut, places=9)
        self.assertAlmostEqual(sum(z.fill for z in out.zones) + out.rest.fill, out.fill, places=9)
        self.assertAlmostEqual(sum(z.covered for z in out.zones) + out.rest.covered, out.covered, places=9)

    def test_overlapping_zones_are_refused(self) -> None:
        s = step("Üst üste binen bölgeler")
        doc = drawing()
        before = doc.to_bytes()
        answer = CMD.run(doc, cad.TerrainsCompare.from_json(s["input"]))
        self.assertEqual(answer.error.code, "invalid_analysis")
        self.assertEqual(str(answer.error.path), "zones[1].boundary")
        self.assertIn(s["result"]["error"]["message"], str(answer.error))
        self.assertEqual(doc.to_bytes(), before)


if __name__ == "__main__":
    unittest.main()
