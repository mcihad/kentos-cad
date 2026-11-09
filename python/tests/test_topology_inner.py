"""The neighbours' vertices inside an edge an edit changed (docs/adr/3400; plan/map-cad.md MAP-07) through the SDK, held
by scripts/fixtures/topology_inner_cases.py's own reading (no KentOS code in the expectations).

- Esnet without a selection over A's north-east corner stretches A and B2 (both have the corner); with Topolojik
  düzenleme the T junction B1 and B2 share on A's north edge goes to the half of the turned edge in both, J's end to a
  quarter from the corner: the reading's places.
- Yaya dönüştür on A's north edge puts the junction at the arc's middle and bends B1's and B2's parts of the edge to
  the arc's halves.
- Without Topolojik düzenleme only A changes.
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import topology_inner_cases as tc  # noqa: E402

UIDS = [f"0192f5c0-3400-7000-8000-{i:012x}" for i in range(1, len(tc.ENTITIES) + 1)]


def drawing() -> cad.Document:
    setup = dict(tc.SETUP, version=2, uids=UIDS)
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def places(doc: cad.Document, n: int) -> list[tuple[float, float]]:
    e = doc.entity(UIDS[n - 1]).entity.to_json()
    pts = e.get("pts") or [e["a"], e["b"]]
    return [(p["x"] - tc.X, p["y"] - tc.Y) for p in pts]


def has(pts: list[tuple[float, float]], p: tuple[float, float]) -> bool:
    return any(math.dist(q, p) < 1e-6 for q in pts)


class TopologyInner(unittest.TestCase):
    def test_a_t_junction_two_stretched_objects_share_goes_with_the_edge(self) -> None:
        doc = drawing()
        window = cad.StretchWindow.from_json({"min": tc.P(39, 29), "max": tc.P(41, 31)})
        out = cad.entities.stretch(doc, window=window, dx=4, dy=2, topology=cad.StretchTopology())
        self.assertEqual(list(out.stretched), [UIDS[0], UIDS[2]])
        self.assertEqual(list(out.followed), [UIDS[1], UIDS[10]])
        for n in (2, 3):
            self.assertTrue(has(places(doc, n), (22, 31)), places(doc, n))
        self.assertTrue(has(places(doc, 11), (33, 31.5)), places(doc, 11))

    def test_an_edge_made_an_arc_bends_the_parts_along_it(self) -> None:
        doc = drawing()
        place = cad.SubPlace.from_json({"kind": "mid", "part": 0, "ring": 0, "index": 2})
        out = cad.entities.vertex(doc, uid=UIDS[0], action="arc", place=place, topology=cad.StretchTopology())
        self.assertEqual(list(out.ids), [1, 2, 3, 11])
        self.assertTrue(has(places(doc, 2), (20, 40)), "the arc's middle")
        half = -math.tan(math.atan(0.5) / 2)
        for n in (2, 3):
            bulges = doc.entity(UIDS[n - 1]).entity.to_json()["bulges"]
            self.assertAlmostEqual(bulges[0], half, places=12)

    def test_without_topology_only_the_object_stretches(self) -> None:
        doc = drawing()
        window = cad.StretchWindow.from_json({"min": tc.P(39, 29), "max": tc.P(41, 31)})
        out = cad.entities.stretch(doc, window=window, dx=4, dy=2, uids=[UIDS[0]])
        self.assertEqual(list(out.followed), [])
        self.assertTrue(has(places(doc, 2), (20, 30)), places(doc, 2))


if __name__ == "__main__":
    unittest.main()
