"""Leap-frog trigonometric levelling and the datum transformation's area polygon (docs/adr/5300; plan/map-cad.md
MAP-02), held by the test's own readings.

- A leap-frog setup from R1 to N1: the test works out the height difference from the two sights itself (each
  S·cos z + (1 − k)·(S·sin z)²/2R − t, the fore's less the back's; k 0.13, R 6 371 000 m); with one known height and no
  redundancy N1's adjusted height is R1's plus it; the instrument's height is in neither sight.
- Burgas written in WGS 84 / UTM 35N is in the datum operations' box (Türkiye's) but out of their extent 1237's
  polygon (read from the polygon file with the test's own even-odd rule): `cad.crs.transform` notes it out of the
  operation's area and names the extent.
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

POLYGONS = json.loads((ROOT / "fixtures/crs/v1/area-polygons.json").read_text("utf-8"))
TRANSFORM = json.loads((ROOT / "fixtures/commands/v1/cad.crs.transform.json").read_text("utf-8"))
ADJUST = json.loads((ROOT / "fixtures/commands/v1/cad.survey.adjust.json").read_text("utf-8"))


def inside(rings, x, y) -> bool:
    odd = False
    for r in rings:
        for (x1, y1), (x2, y2) in zip(r, r[1:]):
            if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
                odd = not odd
    return odd


def opened(setup) -> cad.Document:
    d = json.loads(json.dumps(dict(setup, version=2)))
    d["uids"] = [f"0192f5c0-5300-7000-8000-{i + 1:012x}" for i in range(len(d["entities"]))]
    return cad.Document.from_bytes(kr.container(kr.document(d)))


def sight(s, k=0.13, r=6_371_000.0) -> float:
    z = s["zenith"] * math.pi / 200.0
    d = s["distance"] * math.sin(z)
    return s["distance"] * math.cos(z) + (1 - k) * d * d / (2 * r) - s["target"]


class Leapfrog(unittest.TestCase):
    def test_one_setup_gives_the_fore_point_the_back_one_s_height_plus_its_difference(self) -> None:
        doc = opened(ADJUST["setup"])
        back = {"distance": 62.415, "zenith": 101.2345, "target": 1.5}
        fore = {"distance": 58.902, "zenith": 97.1234, "target": 1.5}
        work = {"kind": "level", "method": "trigonometric", "known": [{"name": "R1", "h": 100.0}], "rows": [],
                "leapfrog": [{"from": "R1", "to": "N1", "back": back, "fore": fore}]}
        out = cad.survey.adjust(doc, work=cad.AdjustWork.from_json(work)).to_json()
        dh = sight(fore) - sight(back)
        self.assertAlmostEqual(out["leapfrog"][0]["dh"], dh, delta=1e-9)
        length = 62.415 * math.sin(101.2345 * math.pi / 200) + 58.902 * math.sin(97.1234 * math.pi / 200)
        self.assertAlmostEqual(out["leapfrog"][0]["length"], length, delta=1e-9)
        n1 = next(p for p in out["points"] if p["name"] == "N1")
        self.assertAlmostEqual(n1["h"], 100.0 + dh, delta=1e-9)


class OperationArea(unittest.TestCase):
    def test_burgas_is_in_the_box_but_out_of_extent_1237(self) -> None:
        turkey = next(x for x in POLYGONS["extents"] if x["code"] == 1237)
        self.assertFalse(inside(turkey["rings"], 27.47, 42.5), "Burgas")
        self.assertTrue(inside(turkey["rings"], 32.86, 39.93), "Ankara")
        case = next(c for c in TRANSFORM["cases"] if "Burgaz" in c["name"])
        step = case["steps"][0]
        doc = opened(case.get("setup") or TRANSFORM["setup"])
        out = cad.crs.transform.plan_input(doc, cad.CrsTransform.from_json(step["input"])).to_json()
        self.assertEqual(out["operation"]["area"]["extent"], 1237)
        notes = [n for f in out.get("flagged", []) for n in f["notes"]]
        self.assertIn("outside_operation_area", notes)


if __name__ == "__main__":
    unittest.main()
