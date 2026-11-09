"""`road.terrains.compare` by sections from Python (docs/adr/4340; plan/map-cad.md MAP-15), held by the test's own
closed forms, no surface computed here.

A Harita project: a flat base (z = 0) over the rectangle (−5, −5)–(45, 35) and a comparison over the triangle (0, 0),
(40, 0), (0, 30), z = 1 + 0.05·x: the difference is the plane d = 1 + 0.05·x on the triangle, whose volume is
∫∫ d = 600 + 0.05·(40²·30/6) = 1000 m³ (the exact TIN–TIN, `exact`).

- Sections north–south (azimuth 90°) every 10 m: A(x) = 30·(1 − x/40)·(1 + 0.05·x), quadratic, A″ = −0.075. The
  prismoidal formula (Simpson's on each stretch) is exact for it: 1000; the average end area (the trapezoid's) misses
  by L³/12·A″ a stretch, four stretches: 1000 − 25 = 975.
- Sections east–west (azimuth 0°) every 10 m: with X(y) = 40·(1 − y/30), A(y) = X + 0.025·X², A″ = 0.05·(4/3)²;
  three stretches: 1000 + 3·10³/12·0.05·16/9 = 1022.2…; prismoidal 1000.
- Each section's area is the closed form at its place.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(id_: str, name: str) -> dict:
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": dict(STYLE), "children": []}


def project() -> cad.Document:
    base = [(-5.0, -5.0), (45.0, -5.0), (45.0, 35.0), (-5.0, 35.0)]
    top = [(0.0, 0.0), (40.0, 0.0), (0.0, 30.0)]
    points = [("taban", x, y, 0.0) for x, y in base] + [("ust", x, y, 1.0 + 0.05 * x) for x, y in top]
    entities = [{"kind": "point", "id": k + 1, "layerId": l, "attrs": {}, "p": {"x": x, "y": y}, "z": z}
                for k, (l, x, y, z) in enumerate(points)]
    setup = {
        "format": "kentos.document", "version": 2, "name": "Kesitle hacim",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 1000.0, "workspace": "cad", "cadWork": "map", "drawingFont": "barlow",
                     "road": {"surfaces": [{"id": "t-1", "name": "Taban", "points": {"layers": ["taban"]}},
                                           {"id": "t-2", "name": "Üst", "points": {"layers": ["ust"]}}]}},
        "origin": {"x": 0.0, "y": 0.0}, "layers": [layer("0", "0"), layer("taban", "Taban"), layer("ust", "Üst")],
        "activeLayer": "0", "entities": entities,
        "uids": [f"0192f680-7c3e-7d4a-9b1e-{k + 1:012x}" for k in range(len(entities))],
        "styles": {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def compare(doc: cad.Document, **values) -> object:
    return cad.road.terrains.compare.execute_input(doc, cad.TerrainsCompare.from_json(
        {"base": "Taban", "comparison": "Üst", "method": "sections", "interval": 10.0, **values}))


class CompareSections(unittest.TestCase):
    def test_north_south_sections(self) -> None:
        doc = project()
        r = compare(doc, azimuth=90.0)
        self.assertAlmostEqual(r.exact.fill, 1000.0, delta=1e-9)
        self.assertAlmostEqual(r.fill, 975.0, delta=1e-9)
        self.assertEqual(r.cut, 0.0)
        area = lambda x: 30 * (1 - x / 40) * (1 + 0.05 * x)  # noqa: E731
        self.assertEqual([s.station for s in r.sections], [0.0, 10.0, 20.0, 30.0, 40.0])
        for s in r.sections:
            self.assertAlmostEqual(s.fill_area, area(s.station), delta=1e-9)
        p = compare(doc, azimuth=90.0, formula="prismoidal")
        self.assertAlmostEqual(p.fill, 1000.0, delta=1e-9)

    def test_east_west_sections(self) -> None:
        doc = project()
        r = compare(doc)
        self.assertEqual(r.azimuth, 0.0)
        self.assertAlmostEqual(r.fill, 1000 + 3 * 10 ** 3 / 12 * 0.05 * 16 / 9, delta=1e-9)
        width = lambda y: 40 * (1 - y / 30)  # noqa: E731
        for s in r.sections:
            x = width(s.station)
            self.assertAlmostEqual(s.fill_area, x + 0.025 * x * x, delta=1e-9)
        self.assertAlmostEqual(compare(doc, formula="prismoidal").fill, 1000.0, delta=1e-9)

    def test_a_wrong_azimuth_is_refused(self) -> None:
        with self.assertRaises(cad.CommandError) as said:
            compare(project(), azimuth=360.0)
        self.assertIn("semti", str(said.exception))


if __name__ == "__main__":
    unittest.main()
