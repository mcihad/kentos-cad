"""Yüzeyden kot ver's height over the surface and range (`cad.entities.drape`'s `offset` and `range`, docs/adr/4580;
plan/map-cad.md MAP-06), held by the test's own planes (no KentOS code in the expectations).

The drawing is the shared cases' (fixtures/commands/v1/cad.entities.drape.json): the surface “Zemin” of four corners
A (0, 0, 100), B (100, 0, 104), C (100, 80, 112), D (0, 100, 106) from X0, Y0, its triangles ABC and ACD split by AC
(y = 0.8 x), as the test works them out. By the test's rule (Civil 3D's Relative Elevation to Surface and Partial):

- with a height over the surface every vertex taken is its triangle's height plus it (minus below);
- with a range only the vertices from its first to its last take heights; a break point goes only into an edge
  between them (the polyline (10, 50), (60, 30), (90, 10) crosses AC on its first edge at (45, 36));
- an area's range leaves its hole and its closing edge as they were;
- the range is for one object, not a point, within the vertices.
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
from kentos.cad.types import DrapeRange  # noqa: E402

import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.entities.drape.json").read_text("utf-8"))
X0, Y0 = 500000.0, 4400000.0
A, B, C, D = (0.0, 0.0, 100.0), (100.0, 0.0, 104.0), (100.0, 80.0, 112.0), (0.0, 100.0, 106.0)


def plane(t, x, y):
    (x1, y1, z1), (x2, y2, z2), (x3, y3, z3) = t
    det = (x2 - x1) * (y3 - y1) - (x3 - x1) * (y2 - y1)
    a = ((z2 - z1) * (y3 - y1) - (z3 - z1) * (y2 - y1)) / det
    b = ((x2 - x1) * (z3 - z1) - (x3 - x1) * (z2 - z1)) / det
    return z1 + a * (x - x1) + b * (y - y1)


def ground(x, y):
    """The surface's height at a local place: ABC under AC, ACD over it."""
    return plane((A, B, C) if y <= 0.8 * x else (A, C, D), x, y)


def uid(slot: int) -> str:
    return CASES["setup"]["uids"][slot - 1]


POINT, POLYLINE, AREA = 5, 8, 9


class DrapeRelativeRange(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))

    def entity(self, slot: int) -> dict:
        _, saved = kcad.read(self.doc.to_bytes())
        return dict(zip(saved["uids"], saved["entities"]))[uid(slot)]

    def test_a_point_over_and_under_the_surface(self) -> None:
        for offset in (0.15, -0.5):
            cad.entities.drape(self.doc, uids=[uid(POINT)], surface="Zemin", offset=offset)
            self.assertAlmostEqual(self.entity(POINT)["z"], ground(60.0, 20.0) + offset, delta=1e-9)

    def test_the_range_takes_its_vertices_and_its_edges_only(self) -> None:
        out = cad.entities.drape(self.doc, uids=[uid(POLYLINE)], surface="t-1", breaks=True, offset=0.15,
                                 range=DrapeRange.from_json({"from": 0, "to": 1})).to_json()
        self.assertEqual((out["set"], out["outside"], out["inserted"]), (2, 0, 1))
        e = self.entity(POLYLINE)
        local = [(p["x"] - X0, p["y"] - Y0) for p in e["pts"]]
        self.assertEqual(len(local), 4)
        self.assertAlmostEqual(local[1][0], 45.0, delta=1e-6)
        self.assertAlmostEqual(local[1][1], 36.0, delta=1e-6)
        for k in (0, 1, 2):
            self.assertAlmostEqual(e["zs"][k], ground(*local[k]) + 0.15, delta=1e-9)
        self.assertIsNone(e["zs"][3], "the last vertex is not in the range")
        # The crossing edge out of the range: nothing goes in, the first vertex keeps none.
        doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))
        out = cad.entities.drape(doc, uids=[uid(POLYLINE)], surface="t-1", breaks=True,
                                 range=DrapeRange.from_json({"from": 1, "to": 2})).to_json()
        self.assertEqual(out["inserted"], 0)

    def test_an_area_s_range_leaves_its_hole_and_closing_edge(self) -> None:
        before = self.entity(AREA)
        cad.entities.drape(self.doc, uids=[uid(AREA)], surface="t-1", breaks=True,
                           range=DrapeRange.from_json({"from": 0, "to": 2}))
        e = self.entity(AREA)
        # (70, 10)–(70, 60) crosses AC at (70, 56): one vertex in; the closing edge (30, 60)–(30, 10) is not split.
        self.assertEqual(len(e["pts"]), 5)
        self.assertIsNone(e["zs"][4], "the fourth corner is out of the range")
        self.assertEqual(e["holes"], before["holes"])

    def test_refusals(self) -> None:
        command = cad.command("cad.entities.drape")
        for input_, code, path in [
            ({"uids": [uid(POLYLINE), uid(AREA)], "range": {"from": 0, "to": 1}}, "invalid_input", "range"),
            ({"uids": [uid(POINT)], "range": {"from": 0, "to": 1}}, "not_a_path", "range"),
            ({"uids": [uid(POLYLINE)], "range": {"from": 2, "to": 1}}, "wrong_place", "range"),
            ({"uids": [uid(POLYLINE)], "range": {"from": 0, "to": 3}}, "wrong_place", "range"),
        ]:
            answer = command.run(self.doc, command.input_type.from_json(dict(input_, surface="t-1")), op="plan")
            self.assertEqual((answer.error.code, answer.error.path), (code, path), input_)


if __name__ == "__main__":
    unittest.main()
