"""EPSG's area of use polygons and the measured refraction coefficient (docs/adr/4900; plan/map-cad.md MAP-02), held by
the test's own readings.

- A point in the Sea of Marmara written in TUREF / TM30 is in its area of use's box but out of its polygon: the test
  reads the polygon file itself (fixtures/crs/v1/area-polygons.json, the EPSG Dataset's) with its own even-odd rule;
  `cad.crs.transform` notes the point out of the source's area; a point on Kütahya's plain is not noted.
- A traverse whose legs are sighted back: the measured k is the test's own mean of the pairs' coefficients;
  `cad.settings.set`'s `refraction` writes it, and the saved file, read by the independent reader, carries it.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402

import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402

POLYGONS = json.loads((ROOT / "fixtures/crs/v1/area-polygons.json").read_text("utf-8"))
TRANSFORM = json.loads((ROOT / "fixtures/commands/v1/cad.crs.transform.json").read_text("utf-8"))
SURVEY = json.loads((ROOT / "fixtures/commands/v1/cad.survey.points.json").read_text("utf-8"))


def inside(rings, x, y) -> bool:
    """The test's own even-odd rule."""
    odd = False
    for r in rings:
        for (x1, y1), (x2, y2) in zip(r, r[1:]):
            if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
                odd = not odd
    return odd


def opened(cases, setup=None) -> cad.Document:
    """The cases' drawing (version 1) as a version 2 document with lasting ids."""
    d = json.loads(json.dumps(dict(setup or cases["setup"], version=2)))
    d["uids"] = [f"0192f5c0-4900-7000-8000-{i + 1:012x}" for i in range(len(d["entities"]))]
    return cad.Document.from_bytes(kr.container(kr.document(d)))


class AreaPolygon(unittest.TestCase):
    def test_the_sea_of_marmara_is_in_tm30_s_box_but_not_its_polygon(self) -> None:
        tm30 = next(x for x in POLYGONS["extents"] if 5254 in x["srids"])
        self.assertFalse(inside(tm30["rings"], 28.9, 40.75), "Marmara")
        self.assertTrue(inside(tm30["rings"], 29.5, 39.5), "Kütahya's plain")
        case = next(c for c in TRANSFORM["cases"] if "Marmara" in c["name"])
        step = case["steps"][0]
        out = cad.crs.transform.plan_input(opened(TRANSFORM, case.get("setup")), cad.CrsTransform.from_json(step["input"]))
        flagged = out.to_json().get("flagged", [])
        self.assertEqual([(f["index"], f["notes"]) for f in flagged], [(1, ["outside_source_area"])])


class MeasuredK(unittest.TestCase):
    def test_the_pairs_mean_is_written_to_the_project(self) -> None:
        case = next(c for c in SURVEY["cases"] if c["name"].startswith("Poligon hesabı karşılıklı başucu açılarıyla: havanın"))
        doc = opened(SURVEY, case.get("setup"))
        out = cad.survey.points.execute_input(doc, cad.SurveyPoints.from_json(case["steps"][0]["input"])).to_json()
        ks = [k for k in out["refractions"] if k is not None]
        mine = sum(ks) / len(ks)
        self.assertAlmostEqual(out["refraction"]["mean"], mine, delta=1e-12)
        self.assertEqual(out["refraction"]["pairs"], len(ks))
        cad.settings.set.execute_input(doc, cad.SettingsSet.from_json({"refraction": out["refraction"]["mean"]}))
        _, saved = kcad.read(doc.to_bytes())
        self.assertAlmostEqual(saved["settings"]["survey"]["refraction"], mine, delta=1e-12)


if __name__ == "__main__":
    unittest.main()
